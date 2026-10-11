mod discovery;

use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    error, get,
    http::{header, KeepAlive},
    middleware::{self, Next},
    web, App, Error, HttpResponse, HttpServer,
};
use chrono::{DateTime, Utc};
use hiking_db::models::ElevationPoint;
use serde::{Deserialize, Serialize};
use sqlx::{Executor, FromRow, PgPool, Postgres};
use std::collections::HashMap;
use std::fmt::{self, Display};
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use uuid::Uuid;

const CACHE_SEARCH: &str = "public, max-age=30, s-maxage=300, stale-while-revalidate=3600";
const CACHE_TRAILS: &str = "public, max-age=30, s-maxage=60, stale-while-revalidate=120";
const CACHE_ELEVATION: &str = "public, max-age=3600, s-maxage=86400, stale-while-revalidate=604800";
const MAX_TRAIL_LIST_OFFSET: i64 = 10_000;
const DEFAULT_READINESS_TIMEOUT_MS: u64 = 1_000;
const MAX_READINESS_TIMEOUT_MS: u64 = 5_000;
const DEFAULT_EXPORT_MAX_POINTS: usize = 100_000;
const ABSOLUTE_EXPORT_MAX_POINTS: usize = 100_000;
const DEFAULT_EXPORT_MAX_BODY_BYTES: usize = 8 * 1024 * 1024;
const ABSOLUTE_EXPORT_MAX_BODY_BYTES: usize = 8 * 1024 * 1024;
const MAX_TRAIL_SURFACE_ROWS: i64 = 256;
const MAX_TRAIL_FEATURE_ROWS: i64 = 256;
const MAX_TRAIL_WAYPOINT_ROWS: i64 = 2_000;
const MAX_TRAIL_DIAGNOSTIC_ROWS: i64 = 2_000;
const MAX_TRAIL_WARNING_ROWS: i64 = 500;

#[derive(Debug, Clone)]
struct ApiConfig {
    bind_addr: String,
    workers: usize,
    max_connections: usize,
    backlog: u32,
    keep_alive: Duration,
    client_request_timeout: Duration,
    client_disconnect_timeout: Duration,
    shutdown_timeout_seconds: u64,
    request_timeout: Duration,
    queue_timeout: Duration,
    readiness_timeout: Duration,
    max_in_flight_requests: usize,
    export_limits: ExportLimits,
}

impl ApiConfig {
    fn from_env() -> anyhow::Result<Self> {
        let max_in_flight_requests = env_usize("API_MAX_IN_FLIGHT_REQUESTS", 64)?;
        if max_in_flight_requests == 0 {
            anyhow::bail!("API_MAX_IN_FLIGHT_REQUESTS must be at least 1");
        }
        let readiness_timeout_ms =
            env_u64("API_READINESS_TIMEOUT_MS", DEFAULT_READINESS_TIMEOUT_MS)?;
        if readiness_timeout_ms == 0 || readiness_timeout_ms > MAX_READINESS_TIMEOUT_MS {
            anyhow::bail!(
                "API_READINESS_TIMEOUT_MS must be between 1 and {MAX_READINESS_TIMEOUT_MS}"
            );
        }

        // Operators may lower these ceilings, but runtime configuration cannot
        // raise them beyond the memory-safe limits compiled into the API.
        let export_limits = ExportLimits {
            max_points: bounded_env_usize(
                "API_EXPORT_MAX_POINTS",
                DEFAULT_EXPORT_MAX_POINTS,
                ABSOLUTE_EXPORT_MAX_POINTS,
            )?,
            max_body_bytes: bounded_env_usize(
                "API_EXPORT_MAX_BODY_BYTES",
                DEFAULT_EXPORT_MAX_BODY_BYTES,
                ABSOLUTE_EXPORT_MAX_BODY_BYTES,
            )?,
        };

        Ok(Self {
            bind_addr: std::env::var("API_BIND_ADDR")
                .ok()
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "0.0.0.0:8080".to_string()),
            workers: env_usize("API_WORKERS", 2)?.max(1),
            max_connections: env_usize("API_MAX_CONNECTIONS", 1_024)?.max(1),
            backlog: env_u32("API_BACKLOG", 256)?.max(1),
            keep_alive: Duration::from_secs(env_u64("API_KEEP_ALIVE_SECONDS", 15)?),
            client_request_timeout: Duration::from_secs(env_u64(
                "API_CLIENT_REQUEST_TIMEOUT_SECONDS",
                10,
            )?),
            client_disconnect_timeout: Duration::from_secs(env_u64(
                "API_CLIENT_DISCONNECT_TIMEOUT_SECONDS",
                5,
            )?),
            shutdown_timeout_seconds: env_u64("API_SHUTDOWN_TIMEOUT_SECONDS", 30)?,
            request_timeout: Duration::from_secs(env_u64("API_REQUEST_TIMEOUT_SECONDS", 30)?),
            queue_timeout: Duration::from_millis(env_u64("API_QUEUE_TIMEOUT_MS", 2_000)?),
            readiness_timeout: Duration::from_millis(readiness_timeout_ms),
            max_in_flight_requests,
            export_limits,
        })
    }
}

#[derive(Debug)]
struct RequestPolicy {
    permits: Arc<Semaphore>,
    readiness_permits: Arc<Semaphore>,
    request_timeout: Duration,
    queue_timeout: Duration,
    readiness_timeout: Duration,
}

impl RequestPolicy {
    fn try_acquire_readiness(&self) -> Option<OwnedSemaphorePermit> {
        Arc::clone(&self.readiness_permits).try_acquire_owned().ok()
    }
}

#[derive(Debug, Clone, Copy)]
struct ExportLimits {
    max_points: usize,
    max_body_bytes: usize,
}

#[derive(Debug, Serialize, FromRow)]
struct TrailSummary {
    id: Uuid,
    name: Option<String>,
    network: Option<String>,
    difficulty: Option<String>,
    distance_m: Option<f64>,
    ascent_m: Option<f64>,
    descent_m: Option<f64>,
    hike_type: Option<String>,
    estimated_duration_min: Option<i32>,
}

#[derive(Debug, FromRow)]
struct TrailDetailRow {
    data_revision: Option<String>,
    id: Uuid,
    osm_relation_id: i64,
    last_synced_at: Option<DateTime<Utc>>,
    cautions: serde_json::Value,
    region_id: Option<Uuid>,
    region_country_code: Option<String>,
    name: Option<String>,
    network: Option<String>,
    difficulty: Option<String>,
    distance_m: Option<f64>,
    ascent_m: Option<f64>,
    descent_m: Option<f64>,
    summary: Option<String>,
    hike_type: Option<String>,
    estimated_duration_min: Option<i32>,
    default_direction_mode: String,
    direction_source: Option<String>,
    source_kind: String,
    network_type: Option<String>,
    route_tag: Option<String>,
    route_from: Option<String>,
    route_to: Option<String>,
    route_via: Option<String>,
    website_url: Option<String>,
    video_url: Option<String>,
    original_mapper_name: Option<String>,
    original_mapper_provider: Option<String>,
    original_mapper_user_id: Option<String>,
    original_mapper_profile_url: Option<String>,
    original_mapped_at: Option<DateTime<Utc>>,
    original_mapper_source: Option<String>,
    geometry_geojson: Option<String>,
    start_point_geojson: Option<String>,
    end_point_geojson: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
struct SurfaceBreakdownRow {
    surface_type: String,
    distance_m: f64,
    pct: f64,
}

#[derive(Debug, Serialize, FromRow)]
struct FeatureRow {
    feature_slug: String,
}

#[derive(Debug, Serialize, FromRow)]
struct WaypointRow {
    kind: String,
    longitude: Option<f64>,
    latitude: Option<f64>,
    name: Option<String>,
    elevation_m: Option<f64>,
}

#[derive(Debug, Serialize, FromRow)]
struct StitchDiagnosticRow {
    kind: String,
    geometry_geojson: String,
}

#[derive(Debug, Serialize, FromRow)]
struct StitchWarningRow {
    message: String,
}

#[derive(Debug, FromRow)]
struct CountryBoundsRow {
    country_code: String,
    min_lon: Option<f64>,
    min_lat: Option<f64>,
    max_lon: Option<f64>,
    max_lat: Option<f64>,
}

#[derive(Debug, Serialize, FromRow)]
struct ElevationRow {
    data_revision: Option<String>,
    min_elevation_m: Option<f64>,
    max_elevation_m: Option<f64>,
    profile_points: Option<serde_json::Value>,
}

#[derive(Debug, FromRow)]
struct TrailExportRow {
    data_revision: Option<String>,
    name: Option<String>,
    has_geometry: bool,
    geometry_point_count: Option<i64>,
    profile_point_count: Option<i64>,
    geometry_size_bytes: Option<i64>,
    profile_size_bytes: Option<i64>,
    geometry: Option<sqlx::types::Json<TrailGeometry>>,
    profile_points: Option<sqlx::types::Json<Vec<ElevationPoint>>>,
}

#[derive(Debug, Deserialize)]
struct DownloadQuery {
    revision: Option<String>,
    format: Option<String>,
    direction: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RevisionQuery {
    revision: Option<String>,
}

fn revision_changed_response() -> HttpResponse {
    HttpResponse::Conflict()
        .insert_header((header::CACHE_CONTROL, "no-store"))
        .json(
            serde_json::json!({ "error": "route changed; reload its overview before continuing" }),
        )
}

#[derive(Debug, Deserialize)]
struct TrailListQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    q: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct TrailWarningPayload {
    kind: String,
    message: String,
}

#[derive(Debug, FromRow)]
struct SearchTrailRow {
    id: Uuid,
    name: Option<String>,
    network: Option<String>,
    distance_m: Option<f64>,
    region_id: Option<Uuid>,
    region_country_code: Option<String>,
    route_from: Option<String>,
    route_to: Option<String>,
    route_via: Option<String>,
    summary: Option<String>,
    _score: f64,
}

#[derive(Debug, FromRow)]
struct SearchRegionRow {
    id: Uuid,
    name: String,
    admin_level: Option<i16>,
    country_code: Option<String>,
    min_lon: Option<f64>,
    min_lat: Option<f64>,
    max_lon: Option<f64>,
    max_lat: Option<f64>,
    _score: f64,
}

#[derive(Debug, FromRow)]
struct SearchPlaceRow {
    id: Uuid,
    name: String,
    place_kind: String,
    region_id: Option<Uuid>,
    country_code: Option<String>,
    lon: f64,
    lat: f64,
    _score: f64,
}

#[derive(Debug, Serialize)]
struct SearchRouteResult {
    kind: &'static str,
    id: Uuid,
    name: String,
    network: Option<String>,
    distance_m: Option<f64>,
    subtitle: Option<String>,
    breadcrumb: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SearchPlaceResult {
    kind: &'static str,
    id: String,
    name: String,
    place_kind: Option<String>,
    admin_label: Option<String>,
    breadcrumb: Vec<String>,
    bbox: Option<[f64; 4]>,
    center: Option<[f64; 2]>,
}

#[derive(Debug)]
struct SearchCountryRow {
    country_code: String,
    bbox: Option<[f64; 4]>,
    score: f64,
}

#[derive(Debug, Serialize)]
struct SearchResponse {
    routes: Vec<SearchRouteResult>,
    places: Vec<SearchPlaceResult>,
}

#[derive(Debug, Clone, Copy)]
enum ExportFormat {
    Gpx,
    Kml,
    GeoJson,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DownloadDirection {
    Forward,
    Backward,
    Both,
}

#[derive(Debug, Clone, Copy)]
struct ExportPoint {
    lon: f64,
    lat: f64,
    ele: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum TrailGeometry {
    LineString { coordinates: Vec<[f64; 2]> },
    MultiLineString { coordinates: Vec<Vec<[f64; 2]>> },
    GeometryCollection { geometries: Vec<TrailGeometry> },
}

impl ExportFormat {
    fn parse(value: Option<&str>) -> Self {
        match value.unwrap_or("gpx").to_ascii_lowercase().as_str() {
            "kml" => Self::Kml,
            "geojson" => Self::GeoJson,
            _ => Self::Gpx,
        }
    }

    fn content_type(self) -> &'static str {
        match self {
            Self::Gpx => "application/gpx+xml; charset=utf-8",
            Self::Kml => "application/vnd.google-earth.kml+xml; charset=utf-8",
            Self::GeoJson => "application/geo+json; charset=utf-8",
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Self::Gpx => "gpx",
            Self::Kml => "kml",
            Self::GeoJson => "geojson",
        }
    }
}

impl DownloadDirection {
    fn parse(value: Option<&str>) -> Self {
        match value.unwrap_or("forward").to_ascii_lowercase().as_str() {
            "backward" => Self::Backward,
            "both" => Self::Both,
            _ => Self::Forward,
        }
    }

    fn track_count(self) -> usize {
        match self {
            Self::Both => 2,
            Self::Forward | Self::Backward => 1,
        }
    }
}

impl TrailGeometry {
    fn collect_line_strings<'a>(&'a self, output: &mut Vec<&'a [[f64; 2]]>) {
        match self {
            Self::LineString { coordinates } => output.push(coordinates),
            Self::MultiLineString { coordinates } => {
                output.extend(coordinates.iter().map(Vec::as_slice));
            }
            Self::GeometryCollection { geometries } => {
                for geometry in geometries {
                    geometry.collect_line_strings(output);
                }
            }
        }
    }

    fn line_strings(&self) -> Vec<&[[f64; 2]]> {
        let mut output = Vec::new();
        self.collect_line_strings(&mut output);
        output
    }

    fn point_count(&self) -> Option<usize> {
        match self {
            Self::LineString { coordinates } => Some(coordinates.len()),
            Self::MultiLineString { coordinates } => coordinates
                .iter()
                .try_fold(0_usize, |total, line| total.checked_add(line.len())),
            Self::GeometryCollection { geometries } => {
                geometries.iter().try_fold(0_usize, |total, geometry| {
                    total.checked_add(geometry.point_count()?)
                })
            }
        }
    }
}

#[derive(Debug)]
enum ExportBuildError {
    BodyTooLarge,
    Serialization(serde_json::Error),
}

impl Display for ExportBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BodyTooLarge => formatter.write_str("export body exceeds configured limit"),
            Self::Serialization(error) => write!(formatter, "serializing export: {error}"),
        }
    }
}

impl std::error::Error for ExportBuildError {}

#[derive(Debug)]
struct BoundedBody {
    bytes: Vec<u8>,
    max_bytes: usize,
    limit_exceeded: bool,
}

impl BoundedBody {
    fn new(max_bytes: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(max_bytes.min(16 * 1024)),
            max_bytes,
            limit_exceeded: false,
        }
    }

    fn try_extend(&mut self, bytes: &[u8]) -> bool {
        let within_limit = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .is_some_and(|new_len| new_len <= self.max_bytes);
        if within_limit {
            self.bytes.extend_from_slice(bytes);
        } else {
            self.limit_exceeded = true;
        }
        within_limit
    }

    fn append_str(&mut self, value: &str) -> Result<(), ExportBuildError> {
        if self.try_extend(value.as_bytes()) {
            Ok(())
        } else {
            Err(ExportBuildError::BodyTooLarge)
        }
    }

    fn append_fmt(&mut self, arguments: fmt::Arguments<'_>) -> Result<(), ExportBuildError> {
        fmt::write(self, arguments).map_err(|_| ExportBuildError::BodyTooLarge)
    }

    fn append_json<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), ExportBuildError> {
        if let Err(error) = serde_json::to_writer(&mut *self, value) {
            if self.limit_exceeded {
                Err(ExportBuildError::BodyTooLarge)
            } else {
                Err(ExportBuildError::Serialization(error))
            }
        } else {
            Ok(())
        }
    }

    fn len(&self) -> usize {
        self.bytes.len()
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

impl fmt::Write for BoundedBody {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        if self.try_extend(value.as_bytes()) {
            Ok(())
        } else {
            Err(fmt::Error)
        }
    }
}

impl io::Write for BoundedBody {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.try_extend(bytes) {
            Ok(bytes.len())
        } else {
            Err(io::Error::other("export body exceeds configured limit"))
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn max_source_export_points(limits: ExportLimits, direction: DownloadDirection) -> usize {
    limits.max_points / direction.track_count()
}

fn emitted_export_point_count(
    geometry: &TrailGeometry,
    direction: DownloadDirection,
) -> Option<usize> {
    geometry.point_count()?.checked_mul(direction.track_count())
}

fn export_source_size_within_limit(
    geometry_size_bytes: i64,
    profile_size_bytes: Option<i64>,
    max_bytes: usize,
) -> bool {
    let Ok(geometry_size_bytes) = usize::try_from(geometry_size_bytes) else {
        return false;
    };
    let Ok(profile_size_bytes) = usize::try_from(profile_size_bytes.unwrap_or(0)) else {
        return false;
    };

    geometry_size_bytes
        .checked_add(profile_size_bytes)
        .is_some_and(|source_size_bytes| source_size_bytes <= max_bytes)
}

fn export_too_large_response() -> HttpResponse {
    HttpResponse::PayloadTooLarge()
        .insert_header((header::CACHE_CONTROL, "no-store"))
        .json(serde_json::json!({ "error": "trail export exceeds the configured size limit" }))
}

fn internal_error(context: &'static str, error: impl Display) -> actix_web::Error {
    tracing::error!(context, error = %error, "request failed");
    actix_web::error::ErrorInternalServerError("internal server error")
}

fn env_u64(key: &str, default: u64) -> anyhow::Result<u64> {
    match std::env::var(key) {
        Ok(value) if !value.trim().is_empty() => value
            .trim()
            .parse::<u64>()
            .map_err(|error| anyhow::anyhow!("invalid {key}: {error}")),
        _ => Ok(default),
    }
}

fn env_u32(key: &str, default: u32) -> anyhow::Result<u32> {
    let value = env_u64(key, default as u64)?;
    u32::try_from(value).map_err(|_| anyhow::anyhow!("{key} is too large"))
}

fn env_usize(key: &str, default: usize) -> anyhow::Result<usize> {
    let value = env_u64(key, default as u64)?;
    usize::try_from(value).map_err(|_| anyhow::anyhow!("{key} is too large"))
}

fn bounded_env_usize(key: &str, default: usize, absolute_max: usize) -> anyhow::Result<usize> {
    let value = env_usize(key, default)?;
    if value == 0 || value > absolute_max {
        anyhow::bail!("{key} must be between 1 and {absolute_max}");
    }
    Ok(value)
}

fn bypass_normal_request_policy(path: &str) -> bool {
    matches!(path, "/health" | "/ready")
}

async fn enforce_request_policy<B: MessageBody + 'static>(
    policy: web::Data<RequestPolicy>,
    request: ServiceRequest,
    next: Next<B>,
) -> Result<ServiceResponse<B>, Error> {
    // Liveness must remain available even when all normal request permits are
    // occupied. Readiness uses its own one-permit guard and database timeout.
    if bypass_normal_request_policy(request.path()) {
        return next.call(request).await;
    }

    let permit = tokio::time::timeout(
        policy.queue_timeout,
        Arc::clone(&policy.permits).acquire_owned(),
    )
    .await
    .map_err(|_| error::ErrorServiceUnavailable("server is busy"))?
    .map_err(|_| error::ErrorServiceUnavailable("server is shutting down"))?;

    let response = tokio::time::timeout(policy.request_timeout, next.call(request))
        .await
        .map_err(|_| error::ErrorGatewayTimeout("request timed out"))?;
    drop(permit);
    response
}

fn cached_json<T: Serialize>(value: T, cache_control: &'static str) -> HttpResponse {
    HttpResponse::Ok()
        .insert_header((header::CACHE_CONTROL, cache_control))
        .json(value)
}

fn list_limit(query: &TrailListQuery) -> i64 {
    query.limit.unwrap_or(250).clamp(1, 1_000)
}

fn list_offset(query: &TrailListQuery) -> i64 {
    query.offset.unwrap_or(0).clamp(0, MAX_TRAIL_LIST_OFFSET)
}

fn search_limit(query: &SearchQuery) -> i64 {
    query.limit.unwrap_or(12).clamp(1, 20)
}

fn search_term_is_eligible(term: &str) -> bool {
    term.chars().count() >= 3
}

fn escape_ilike_pattern(term: &str) -> String {
    let mut escaped = String::with_capacity(term.len());
    for character in term.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '%' => escaped.push_str("\\%"),
            '_' => escaped.push_str("\\_"),
            _ => escaped.push(character),
        }
    }
    escaped
}

fn admin_level_label(country_code: Option<&str>, admin_level: Option<i16>) -> Option<&'static str> {
    if country_code == Some("IN") {
        return match admin_level {
            Some(2) => Some("Country"),
            Some(4) => Some("State"),
            Some(5) => Some("District"),
            Some(6) => Some("Subdistrict"),
            Some(7) => Some("Local area"),
            Some(8) => Some("Local area"),
            _ => None,
        };
    }

    match admin_level {
        Some(2) => Some("Country"),
        Some(3) => Some("Region"),
        Some(4) => Some("State or province"),
        Some(5) => Some("Division"),
        Some(6) => Some("District"),
        Some(7) => Some("Subdistrict"),
        Some(8) => Some("Local area"),
        _ => None,
    }
}

fn country_name_for_code(code: &str) -> Option<&'static str> {
    match code {
        "IN" => Some("India"),
        "NP" => Some("Nepal"),
        "BT" => Some("Bhutan"),
        "PK" => Some("Pakistan"),
        "CN" => Some("China"),
        _ => None,
    }
}

fn prepend_country_to_breadcrumb(path: &mut Vec<String>, country_code: Option<&str>) {
    let Some(country_name) = country_code.and_then(country_name_for_code) else {
        return;
    };

    if path.first().map(String::as_str) == Some(country_name) {
        return;
    }

    path.retain(|entry| entry != country_name);
    path.insert(0, country_name.to_string());
}

fn country_match_score(code: &str, term: &str) -> f64 {
    let normalized_term = term.trim().to_ascii_lowercase();
    if normalized_term.is_empty() {
        return 0.0;
    }

    let normalized_code = code.to_ascii_lowercase();
    let normalized_name = country_name_for_code(code)
        .map(str::to_ascii_lowercase)
        .unwrap_or_else(|| normalized_code.clone());

    if normalized_term == normalized_name {
        return 1200.0;
    }

    if normalized_term == normalized_code {
        return 1100.0;
    }

    if normalized_name.starts_with(&normalized_term) {
        return 930.0;
    }

    if normalized_code.starts_with(&normalized_term) {
        return 840.0;
    }

    if normalized_name.contains(&normalized_term) {
        return 760.0;
    }

    if normalized_code.contains(&normalized_term) {
        return 680.0;
    }

    0.0
}

#[derive(Debug, FromRow)]
struct RegionChainRow {
    root_id: Uuid,
    name: String,
    _depth: i32,
}

async fn fetch_region_paths<'e, E>(
    executor: E,
    region_ids: &[Uuid],
) -> Result<HashMap<Uuid, Vec<String>>, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    if region_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let rows = sqlx::query_as::<_, RegionChainRow>(
        r#"
        WITH RECURSIVE seed AS (
            SELECT DISTINCT unnest($1::uuid[]) AS root_id
        ),
        chain AS (
            SELECT seed.root_id, regions.id, regions.name, regions.parent_region_id, 0 AS depth
            FROM seed
            JOIN regions ON regions.id = seed.root_id

            UNION ALL

            SELECT chain.root_id, parent.id, parent.name, parent.parent_region_id, chain.depth + 1
            FROM chain
            JOIN regions parent ON parent.id = chain.parent_region_id
        )
        SELECT root_id, name, depth AS _depth
        FROM chain
        ORDER BY root_id, depth DESC
        "#,
    )
    .bind(region_ids)
    .fetch_all(executor)
    .await?;

    let mut paths = HashMap::<Uuid, Vec<String>>::new();
    for row in rows {
        paths.entry(row.root_id).or_default().push(row.name);
    }

    for path in paths.values_mut() {
        let mut deduped = Vec::with_capacity(path.len());
        for name in path.drain(..) {
            if deduped.last() != Some(&name) {
                deduped.push(name);
            }
        }
        *path = deduped;
    }

    Ok(paths)
}

fn classify_warning_kind(message: &str) -> &'static str {
    if message.contains("branch point at a shared node") {
        "branch_ambiguity"
    } else if message.contains("relation member way(s) were missing from raw_ways") {
        "missing_members"
    } else if message.contains("relation splits into") {
        "disconnected_pieces"
    } else if message.contains("could not be reached by a simple walk") {
        "stranded_segments"
    } else {
        "generic"
    }
}

fn approx_distance_m(a: [f64; 2], b: [f64; 2]) -> f64 {
    let lon_scale = ((a[1] + b[1]) * 0.5).to_radians().cos();
    let dx = (b[0] - a[0]) * 111_320.0 * lon_scale;
    let dy = (b[1] - a[1]) * 110_540.0;
    (dx * dx + dy * dy).sqrt()
}

fn total_route_distance_m(line_strings: &[&[[f64; 2]]]) -> f64 {
    line_strings
        .iter()
        .flat_map(|line| line.windows(2))
        .map(|pair| approx_distance_m(pair[0], pair[1]))
        .sum()
}

struct ElevationCursor<'a> {
    profile_points: &'a [ElevationPoint],
    right_index: usize,
    reversed: bool,
    total_distance_m: f64,
}

impl<'a> ElevationCursor<'a> {
    fn new(profile_points: &'a [ElevationPoint]) -> Self {
        Self {
            profile_points,
            right_index: usize::from(profile_points.len() > 1),
            reversed: false,
            total_distance_m: 0.0,
        }
    }

    fn reversed(profile_points: &'a [ElevationPoint], total_distance_m: f64) -> Self {
        Self {
            profile_points,
            right_index: usize::from(profile_points.len() > 1),
            reversed: true,
            total_distance_m,
        }
    }

    fn point_at(&self, index: usize) -> Option<(f64, f64)> {
        let source_index = if self.reversed {
            self.profile_points
                .len()
                .checked_sub(index.checked_add(1)?)?
        } else {
            index
        };
        let point = self.profile_points.get(source_index)?;
        let pos_m = if self.reversed {
            (self.total_distance_m - point.pos_m).max(0.0)
        } else {
            point.pos_m
        };
        Some((pos_m, point.ele_m))
    }

    fn elevation_at(&mut self, pos_m: f64) -> Option<f64> {
        let (first_pos_m, first_ele_m) = self.point_at(0)?;
        if pos_m <= first_pos_m {
            return Some(first_ele_m);
        }

        while self.right_index < self.profile_points.len()
            && pos_m > self.point_at(self.right_index)?.0
        {
            self.right_index += 1;
        }

        let Some((right_pos_m, right_ele_m)) = self.point_at(self.right_index) else {
            return self
                .point_at(self.profile_points.len().saturating_sub(1))
                .map(|(_, ele_m)| ele_m);
        };
        let (left_pos_m, left_ele_m) = self.point_at(self.right_index.saturating_sub(1))?;
        let span = (right_pos_m - left_pos_m).max(f64::EPSILON);
        let fraction = ((pos_m - left_pos_m) / span).clamp(0.0, 1.0);
        Some(left_ele_m + (right_ele_m - left_ele_m) * fraction)
    }
}

#[derive(Debug, Clone, Copy)]
enum ExportEvent {
    TrackStart {
        label: &'static str,
        line_count: usize,
    },
    LineStart,
    Point(ExportPoint),
    LineEnd,
    TrackEnd,
}

fn visit_export_line<I, E, F>(
    coordinates: I,
    cumulative_m: &mut f64,
    elevation_cursor: &mut Option<ElevationCursor<'_>>,
    visitor: &mut F,
) -> Result<(), E>
where
    I: Iterator<Item = [f64; 2]>,
    F: FnMut(ExportEvent) -> Result<(), E>,
{
    visitor(ExportEvent::LineStart)?;
    let mut last_coord: Option<[f64; 2]> = None;

    for coord in coordinates {
        if let Some(previous) = last_coord {
            *cumulative_m += approx_distance_m(previous, coord);
        }
        last_coord = Some(coord);
        visitor(ExportEvent::Point(ExportPoint {
            lon: coord[0],
            lat: coord[1],
            ele: elevation_cursor
                .as_mut()
                .and_then(|cursor| cursor.elevation_at(*cumulative_m)),
        }))?;
    }

    visitor(ExportEvent::LineEnd)
}

fn visit_export_track<E, F>(
    label: &'static str,
    line_string_refs: &[&[[f64; 2]]],
    profile_points: Option<&[ElevationPoint]>,
    reversed: bool,
    total_distance_m: f64,
    visitor: &mut F,
) -> Result<(), E>
where
    F: FnMut(ExportEvent) -> Result<(), E>,
{
    visitor(ExportEvent::TrackStart {
        label,
        line_count: line_string_refs
            .iter()
            .filter(|line| !line.is_empty())
            .count(),
    })?;

    let mut cumulative_m = 0.0;
    let mut elevation_cursor = profile_points.map(|points| {
        if reversed {
            ElevationCursor::reversed(points, total_distance_m)
        } else {
            ElevationCursor::new(points)
        }
    });

    if reversed {
        for line in line_string_refs.iter().rev() {
            if !line.is_empty() {
                visit_export_line(
                    line.iter().rev().copied(),
                    &mut cumulative_m,
                    &mut elevation_cursor,
                    visitor,
                )?;
            }
        }
    } else {
        for line in line_string_refs {
            if !line.is_empty() {
                visit_export_line(
                    line.iter().copied(),
                    &mut cumulative_m,
                    &mut elevation_cursor,
                    visitor,
                )?;
            }
        }
    }

    visitor(ExportEvent::TrackEnd)
}

fn visit_export_tracks<E, F>(
    geometry: &TrailGeometry,
    profile_points: Option<&[ElevationPoint]>,
    direction: DownloadDirection,
    mut visitor: F,
) -> Result<(), E>
where
    F: FnMut(ExportEvent) -> Result<(), E>,
{
    let line_string_refs = geometry.line_strings();

    match direction {
        DownloadDirection::Forward => visit_export_track(
            "forward",
            &line_string_refs,
            profile_points,
            false,
            0.0,
            &mut visitor,
        ),
        DownloadDirection::Backward | DownloadDirection::Both => {
            let total_distance_m = total_route_distance_m(&line_string_refs);
            if direction == DownloadDirection::Both {
                visit_export_track(
                    "forward",
                    &line_string_refs,
                    profile_points,
                    false,
                    total_distance_m,
                    &mut visitor,
                )?;
            }

            visit_export_track(
                "backward",
                &line_string_refs,
                profile_points,
                true,
                total_distance_m,
                &mut visitor,
            )
        }
    }
}

fn slugify_filename(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    let mut last_dash = false;

    for ch in name.chars() {
        let lowered = ch.to_ascii_lowercase();
        if lowered.is_ascii_alphanumeric() {
            slug.push(lowered);
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }

    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "trail-route".to_string()
    } else {
        slug.to_string()
    }
}

fn write_xml_escaped(body: &mut BoundedBody, value: &str) -> Result<(), ExportBuildError> {
    let mut encoded = [0_u8; 4];
    for character in value.chars() {
        match character {
            '&' => body.append_str("&amp;")?,
            '<' => body.append_str("&lt;")?,
            '>' => body.append_str("&gt;")?,
            '"' => body.append_str("&quot;")?,
            '\'' => body.append_str("&apos;")?,
            _ => body.append_str(character.encode_utf8(&mut encoded))?,
        }
    }
    Ok(())
}

fn build_geojson_export(
    name: &str,
    geometry: &TrailGeometry,
    profile_points: Option<&[ElevationPoint]>,
    direction: DownloadDirection,
    max_body_bytes: usize,
) -> Result<Vec<u8>, ExportBuildError> {
    let mut body = BoundedBody::new(max_body_bytes);
    body.append_str("{\"type\":\"FeatureCollection\",\"features\":[")?;

    let mut first_feature = true;
    let mut first_line = true;
    let mut first_point = true;
    let mut multi_line = false;
    visit_export_tracks(geometry, profile_points, direction, |event| {
        match event {
            ExportEvent::TrackStart { label, line_count } => {
                if !first_feature {
                    body.append_str(",")?;
                }
                first_feature = false;
                first_line = true;
                multi_line = line_count != 1;

                body.append_str("{\"type\":\"Feature\",\"properties\":{\"name\":")?;
                body.append_json(name)?;
                body.append_str(",\"direction\":")?;
                body.append_json(label)?;
                body.append_str("},\"geometry\":{\"type\":\"")?;
                body.append_str(if multi_line {
                    "MultiLineString"
                } else {
                    "LineString"
                })?;
                body.append_str("\",\"coordinates\":[")?;
            }
            ExportEvent::LineStart => {
                if multi_line {
                    if !first_line {
                        body.append_str(",")?;
                    }
                    body.append_str("[")?;
                }
                first_line = false;
                first_point = true;
            }
            ExportEvent::Point(point) => {
                if !first_point {
                    body.append_str(",")?;
                }
                first_point = false;
                if let Some(ele) = point.ele {
                    body.append_json(&[point.lon, point.lat, ele])?;
                } else {
                    body.append_json(&[point.lon, point.lat])?;
                }
            }
            ExportEvent::LineEnd => {
                if multi_line {
                    body.append_str("]")?;
                }
            }
            ExportEvent::TrackEnd => body.append_str("]}}")?,
        }
        Ok(())
    })?;

    body.append_str("]}")?;
    debug_assert!(body.len() <= max_body_bytes);
    Ok(body.finish())
}

fn build_gpx_export(
    name: &str,
    geometry: &TrailGeometry,
    profile_points: Option<&[ElevationPoint]>,
    direction: DownloadDirection,
    max_body_bytes: usize,
) -> Result<Vec<u8>, ExportBuildError> {
    let mut body = BoundedBody::new(max_body_bytes);
    body.append_str(
        r#"<?xml version="1.0" encoding="UTF-8"?><gpx version="1.1" creator="hiking-trails-app" xmlns="http://www.topografix.com/GPX/1/1">"#,
    )?;

    visit_export_tracks(geometry, profile_points, direction, |event| {
        match event {
            ExportEvent::TrackStart { label, .. } => {
                body.append_str("<trk><name>")?;
                write_xml_escaped(&mut body, name)?;
                body.append_str(" (")?;
                write_xml_escaped(&mut body, label)?;
                body.append_str(")</name>")?;
            }
            ExportEvent::LineStart => body.append_str("<trkseg>")?,
            ExportEvent::Point(point) => {
                body.append_fmt(format_args!(
                    r#"<trkpt lat="{:.7}" lon="{:.7}">"#,
                    point.lat, point.lon
                ))?;
                if let Some(elevation_m) = point.ele {
                    body.append_fmt(format_args!("<ele>{elevation_m:.1}</ele>"))?;
                }
                body.append_str("</trkpt>")?;
            }
            ExportEvent::LineEnd => body.append_str("</trkseg>")?,
            ExportEvent::TrackEnd => body.append_str("</trk>")?,
        }
        Ok(())
    })?;

    body.append_str("</gpx>")?;
    debug_assert!(body.len() <= max_body_bytes);
    Ok(body.finish())
}

fn build_kml_export(
    name: &str,
    geometry: &TrailGeometry,
    profile_points: Option<&[ElevationPoint]>,
    direction: DownloadDirection,
    max_body_bytes: usize,
) -> Result<Vec<u8>, ExportBuildError> {
    let mut body = BoundedBody::new(max_body_bytes);
    body.append_str(
        r#"<?xml version="1.0" encoding="UTF-8"?><kml xmlns="http://www.opengis.net/kml/2.2"><Document>"#,
    )?;
    let mut first_coordinate = true;

    visit_export_tracks(geometry, profile_points, direction, |event| {
        match event {
            ExportEvent::TrackStart { label, .. } => {
                body.append_str("<Placemark><name>")?;
                write_xml_escaped(&mut body, name)?;
                body.append_str(" (")?;
                write_xml_escaped(&mut body, label)?;
                body.append_str(")</name><MultiGeometry>")?;
            }
            ExportEvent::LineStart => {
                first_coordinate = true;
                body.append_str("<LineString><tessellate>1</tessellate><coordinates>")?;
            }
            ExportEvent::Point(point) => {
                if !first_coordinate {
                    body.append_str(" ")?;
                }
                first_coordinate = false;
                if let Some(ele) = point.ele {
                    body.append_fmt(format_args!(
                        "{:.7},{:.7},{:.1}",
                        point.lon,
                        point.lat,
                        ele
                    ))?;
                } else {
                    body.append_fmt(format_args!(
                        "{:.7},{:.7}",
                        point.lon,
                        point.lat,
                    ))?;
                }
            }
            ExportEvent::LineEnd => body.append_str("</coordinates></LineString>")?,
            ExportEvent::TrackEnd => body.append_str("</MultiGeometry></Placemark>")?,
        }
        Ok(())
    })?;

    body.append_str("</Document></kml>")?;
    debug_assert!(body.len() <= max_body_bytes);
    Ok(body.finish())
}

#[get("/health")]
async fn health() -> HttpResponse {
    HttpResponse::Ok()
        .insert_header((header::CACHE_CONTROL, "no-store"))
        .json(serde_json::json!({ "status": "ok" }))
}

#[get("/ready")]
async fn ready(pool: web::Data<PgPool>, policy: web::Data<RequestPolicy>) -> HttpResponse {
    let Some(_readiness_permit) = policy.try_acquire_readiness() else {
        return HttpResponse::ServiceUnavailable()
            .insert_header((header::CACHE_CONTROL, "no-store"))
            .json(serde_json::json!({ "status": "not_ready" }));
    };

    let readiness_check = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT
            to_regclass('public.trails') IS NOT NULL
            AND to_regclass('public.country_bounds') IS NOT NULL
        "#,
    )
    .fetch_one(pool.get_ref());

    match tokio::time::timeout(policy.readiness_timeout, readiness_check).await {
        Ok(Ok(true)) => HttpResponse::Ok()
            .insert_header((header::CACHE_CONTROL, "no-store"))
            .json(serde_json::json!({ "status": "ready" })),
        Ok(Ok(false)) => HttpResponse::ServiceUnavailable()
            .insert_header((header::CACHE_CONTROL, "no-store"))
            .json(serde_json::json!({ "status": "not_ready" })),
        Ok(Err(error)) => {
            tracing::warn!(error = %error, "database readiness check failed");
            HttpResponse::ServiceUnavailable()
                .insert_header((header::CACHE_CONTROL, "no-store"))
                .json(serde_json::json!({ "status": "not_ready" }))
        }
        Err(_) => {
            tracing::warn!(
                timeout_ms = policy.readiness_timeout.as_millis(),
                "database readiness check timed out"
            );
            HttpResponse::ServiceUnavailable()
                .insert_header((header::CACHE_CONTROL, "no-store"))
                .json(serde_json::json!({ "status": "not_ready" }))
        }
    }
}

#[get("/regions/{region_id}/trails")]
async fn list_trails_in_region(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    query: web::Query<TrailListQuery>,
) -> actix_web::Result<HttpResponse> {
    let region_id = path.into_inner();
    let query = query.into_inner();

    let trails = sqlx::query_as::<_, TrailSummary>(
        r#"
        WITH RECURSIVE descendants AS (
            SELECT id FROM regions WHERE id = $1
            UNION
            SELECT r.id FROM regions r JOIN descendants d ON r.parent_region_id = d.id
        )
        SELECT
            id,
            name,
            network,
            difficulty,
            distance_m,
            ascent_m,
            descent_m,
            hike_type,
            estimated_duration_min
        FROM trails
        WHERE region_id IN (SELECT id FROM descendants)
        ORDER BY name, id
        LIMIT $2
        OFFSET $3
        "#,
    )
    .bind(region_id)
    .bind(list_limit(&query))
    .bind(list_offset(&query))
    .fetch_all(pool.get_ref())
    .await
    .map_err(|error| internal_error("listing trails in region", error))?;

    Ok(cached_json(trails, CACHE_TRAILS))
}

#[get("/search")]
async fn search(
    pool: web::Data<PgPool>,
    query: web::Query<SearchQuery>,
) -> actix_web::Result<HttpResponse> {
    let query = query.into_inner();
    let raw_term = query.q.clone().unwrap_or_default();
    let term = raw_term.trim();
    let term_length = term.chars().count();

    if term.is_empty() {
        return Ok(cached_json(
            SearchResponse {
                routes: Vec::new(),
                places: Vec::new(),
            },
            CACHE_SEARCH,
        ));
    }
    if !search_term_is_eligible(term) {
        return Ok(cached_json(
            SearchResponse {
                routes: Vec::new(),
                places: Vec::new(),
            },
            CACHE_SEARCH,
        ));
    }
    if term_length > 128 {
        return Ok(HttpResponse::BadRequest()
            .insert_header((header::CACHE_CONTROL, "no-store"))
            .json(serde_json::json!({ "error": "search query is too long" })));
    }
    let escaped_term = escape_ilike_pattern(term);

    let route_rows = sqlx::query_as::<_, SearchTrailRow>(
        r#"
        SELECT
            id,
            name,
            network,
            distance_m,
            region_id,
            region_country_code,
            route_from,
            route_to,
            route_via,
            summary,
            _score
        FROM (
            SELECT
                trails.id,
                trails.name,
                trails.network,
                trails.distance_m,
                trails.region_id,
                regions.country_code AS region_country_code,
                trails.route_from,
                trails.route_to,
                trails.route_via,
                trails.summary,
                (
                    CASE
                        WHEN lower(COALESCE(trails.name, '')) = lower($1) THEN 1000
                        WHEN lower(COALESCE(trails.name, '')) LIKE lower($2) || '%' ESCAPE '\' THEN 850
                        WHEN COALESCE(trails.name, '') ILIKE '%' || $2 || '%' ESCAPE '\' THEN 700
                        WHEN COALESCE(trails.route_from, '') ILIKE '%' || $2 || '%' ESCAPE '\' THEN 560
                        WHEN COALESCE(trails.route_to, '') ILIKE '%' || $2 || '%' ESCAPE '\' THEN 560
                        WHEN COALESCE(trails.route_via, '') ILIKE '%' || $2 || '%' ESCAPE '\' THEN 540
                        WHEN COALESCE(trails.summary, '') ILIKE '%' || $2 || '%' ESCAPE '\' THEN 480
                        ELSE 0
                    END
                    + (
                        GREATEST(
                            similarity(COALESCE(trails.name, ''), $1),
                            similarity(COALESCE(trails.route_from, ''), $1),
                            similarity(COALESCE(trails.route_to, ''), $1),
                            similarity(COALESCE(trails.route_via, ''), $1),
                            similarity(COALESCE(trails.summary, ''), $1)
                        ) * 100
                    )
                ) AS _score
            FROM trails
            LEFT JOIN regions ON regions.id = trails.region_id
            WHERE
                trails.search_text ILIKE '%' || $2 || '%' ESCAPE '\'
                OR trails.search_text % $1
        ) matches
        ORDER BY _score DESC, name NULLS LAST, id
        LIMIT $3
        "#,
    )
    .bind(term)
    .bind(&escaped_term)
    .bind(search_limit(&query))
    .fetch_all(pool.get_ref())
    .await
    .map_err(|error| internal_error("searching trails", error))?;

    let region_rows = sqlx::query_as::<_, SearchRegionRow>(
        r#"
        SELECT
            id,
            name,
            admin_level,
            country_code,
            ST_XMin(ST_Envelope(boundary)) AS min_lon,
            ST_YMin(ST_Envelope(boundary)) AS min_lat,
            ST_XMax(ST_Envelope(boundary)) AS max_lon,
            ST_YMax(ST_Envelope(boundary)) AS max_lat,
            (
                CASE
                    WHEN lower(name) = lower($1) THEN 1100
                    WHEN lower(name) LIKE lower($2) || '%' ESCAPE '\' THEN 920
                    WHEN name ILIKE '%' || $2 || '%' ESCAPE '\' THEN 760
                    ELSE 0
                END
                + similarity(name, $1) * 100
            ) AS _score
        FROM regions
        WHERE
            name ILIKE '%' || $2 || '%' ESCAPE '\'
            OR name % $1
        ORDER BY _score DESC, name, id
        LIMIT $3
        "#,
    )
    .bind(term)
    .bind(&escaped_term)
    .bind(search_limit(&query))
    .fetch_all(pool.get_ref())
    .await
    .map_err(|error| internal_error("searching regions", error))?;

    let place_rows = sqlx::query_as::<_, SearchPlaceRow>(
        r#"
        SELECT
            id,
            name,
            place_kind,
            region_id,
            country_code,
            ST_X(geom) AS lon,
            ST_Y(geom) AS lat,
            (
                CASE
                    WHEN lower(name) = lower($1) THEN 1050
                    WHEN lower(name) LIKE lower($2) || '%' ESCAPE '\' THEN 880
                    WHEN name ILIKE '%' || $2 || '%' ESCAPE '\' THEN 720
                    ELSE 0
                END
                + similarity(name, $1) * 100
            ) AS _score
        FROM places
        WHERE
            name ILIKE '%' || $2 || '%' ESCAPE '\'
            OR name % $1
        ORDER BY _score DESC, name, id
        LIMIT $3
        "#,
    )
    .bind(term)
    .bind(&escaped_term)
    .bind(search_limit(&query))
    .fetch_all(pool.get_ref())
    .await
    .map_err(|error| internal_error("searching places", error))?;

    let country_bbox_rows = sqlx::query_as::<_, CountryBoundsRow>(
        r#"
        SELECT country_code, min_lon, min_lat, max_lon, max_lat
        FROM country_bounds
        "#,
    )
    .fetch_all(pool.get_ref())
    .await
    .map_err(|error| internal_error("loading country search candidates", error))?;

    let country_rows = country_bbox_rows
        .into_iter()
        .filter_map(|row| {
            let country_code = row.country_code;
            let score = country_match_score(&country_code, term);
            if score <= 0.0 {
                return None;
            }

            Some(SearchCountryRow {
                country_code,
                bbox: match (row.min_lon, row.min_lat, row.max_lon, row.max_lat) {
                    (Some(min_lon), Some(min_lat), Some(max_lon), Some(max_lat)) => {
                        Some([min_lon, min_lat, max_lon, max_lat])
                    }
                    _ => None,
                },
                score,
            })
        })
        .collect::<Vec<_>>();

    let mut region_ids = route_rows
        .iter()
        .filter_map(|row| row.region_id)
        .chain(region_rows.iter().map(|row| row.id))
        .chain(place_rows.iter().filter_map(|row| row.region_id))
        .collect::<Vec<_>>();
    region_ids.sort_unstable();
    region_ids.dedup();

    let region_paths = fetch_region_paths(pool.get_ref(), &region_ids)
        .await
        .map_err(|error| internal_error("building search breadcrumbs", error))?;

    let routes = route_rows
        .into_iter()
        .map(|row| {
            let mut breadcrumb = row
                .region_id
                .and_then(|region_id| region_paths.get(&region_id).cloned())
                .unwrap_or_default();
            prepend_country_to_breadcrumb(&mut breadcrumb, row.region_country_code.as_deref());

            let route_line = [row.route_from, row.route_via, row.route_to]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            let route_subtitle = if route_line.is_empty() {
                None
            } else {
                Some(route_line.join(" -> "))
            };

            SearchRouteResult {
                kind: "trail",
                id: row.id,
                name: row.name.unwrap_or_else(|| "Unnamed".to_string()),
                network: row.network,
                distance_m: row.distance_m,
                subtitle: if breadcrumb.is_empty() {
                    route_subtitle.or(row.summary)
                } else {
                    Some(breadcrumb.join(", "))
                },
                breadcrumb,
            }
        })
        .collect::<Vec<_>>();

    let mut places = region_rows
        .into_iter()
        .map(|row| {
            (
                row._score,
                SearchPlaceResult {
                    kind: "region",
                    id: row.id.to_string(),
                    name: row.name,
                    place_kind: None,
                    admin_label: admin_level_label(row.country_code.as_deref(), row.admin_level)
                        .map(str::to_string),
                    breadcrumb: {
                        let mut full_path = region_paths.get(&row.id).cloned().unwrap_or_default();
                        prepend_country_to_breadcrumb(&mut full_path, row.country_code.as_deref());
                        if full_path.is_empty() {
                            Vec::new()
                        } else {
                            full_path[..full_path.len().saturating_sub(1)].to_vec()
                        }
                    },
                    bbox: match (row.min_lon, row.min_lat, row.max_lon, row.max_lat) {
                        (Some(min_lon), Some(min_lat), Some(max_lon), Some(max_lat)) => {
                            Some([min_lon, min_lat, max_lon, max_lat])
                        }
                        _ => None,
                    },
                    center: None,
                },
            )
        })
        .chain(country_rows.into_iter().map(|row| {
            (
                row.score,
                SearchPlaceResult {
                    kind: "region",
                    id: format!("country:{}", row.country_code),
                    name: country_name_for_code(&row.country_code)
                        .map(str::to_string)
                        .unwrap_or_else(|| row.country_code.clone()),
                    place_kind: None,
                    admin_label: Some("Country".to_string()),
                    breadcrumb: Vec::new(),
                    bbox: row.bbox,
                    center: None,
                },
            )
        }))
        .chain(place_rows.into_iter().map(|row| {
            let mut breadcrumb = row
                .region_id
                .and_then(|region_id| region_paths.get(&region_id).cloned())
                .unwrap_or_default();
            prepend_country_to_breadcrumb(&mut breadcrumb, row.country_code.as_deref());

            (
                row._score,
                SearchPlaceResult {
                    kind: "place",
                    id: row.id.to_string(),
                    name: row.name,
                    place_kind: Some(row.place_kind.replace('_', " ")),
                    admin_label: None,
                    breadcrumb,
                    bbox: None,
                    center: Some([row.lon, row.lat]),
                },
            )
        }))
        .collect::<Vec<_>>();

    places.sort_by(|left, right| {
        right
            .0
            .partial_cmp(&left.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let places = places
        .into_iter()
        .map(|(_, result)| result)
        .take(search_limit(&query) as usize)
        .collect::<Vec<_>>();

    Ok(cached_json(SearchResponse { routes, places }, CACHE_SEARCH))
}

#[get("/trails/{trail_id}")]
async fn get_trail(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> actix_web::Result<HttpResponse> {
    let trail_id = path.into_inner();
    let mut transaction = pool
        .begin()
        .await
        .map_err(|error| internal_error("starting trail detail transaction", error))?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *transaction)
        .await
        .map_err(|error| internal_error("configuring trail detail transaction", error))?;

    let row = sqlx::query_as::<_, TrailDetailRow>(
        r#"
        SELECT
            md5(ST_AsEWKB(trails.geom)::text || trails.updated_at::text) AS data_revision,
            trails.id,
            trails.osm_relation_id,
            trails.last_synced_at,
            trails.cautions,
            trails.region_id,
            regions.country_code AS region_country_code,
            trails.name,
            trails.network,
            trails.difficulty,
            trails.distance_m,
            trails.ascent_m,
            trails.descent_m,
            trails.summary,
            trails.hike_type,
            trails.estimated_duration_min,
            trails.default_direction_mode,
            trails.direction_source,
            trails.source_kind,
            trails.network_type,
            trails.route_tag,
            trails.route_from,
            trails.route_to,
            trails.route_via,
            trails.website_url,
            trails.video_url,
            trails.original_mapper_name,
            trails.original_mapper_provider,
            trails.original_mapper_user_id,
            trails.original_mapper_profile_url,
            trails.original_mapped_at,
            trails.original_mapper_source,
            ST_AsGeoJSON(trails.geom) AS geometry_geojson,
            ST_AsGeoJSON(start_point) AS start_point_geojson,
            ST_AsGeoJSON(end_point) AS end_point_geojson
        FROM trails
        LEFT JOIN regions ON regions.id = trails.region_id
        WHERE trails.id = $1
        "#,
    )
    .bind(trail_id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(|error| internal_error("fetching trail detail row", error))?;

    let Some(row) = row else {
        transaction
            .commit()
            .await
            .map_err(|error| internal_error("committing empty trail detail transaction", error))?;
        return Ok(HttpResponse::NotFound().json(serde_json::json!({ "error": "trail not found" })));
    };

    let surfaces = sqlx::query_as::<_, SurfaceBreakdownRow>(
        r#"
        SELECT surface_type, distance_m, pct
        FROM trail_surface_breakdown
        WHERE trail_id = $1
        ORDER BY distance_m DESC, surface_type
        LIMIT $2
        "#,
    )
    .bind(trail_id)
    .bind(MAX_TRAIL_SURFACE_ROWS)
    .fetch_all(&mut *transaction)
    .await
    .map_err(|error| internal_error("fetching trail surfaces", error))?;

    let features = sqlx::query_as::<_, FeatureRow>(
        r#"
        SELECT feature_slug
        FROM trail_features
        WHERE trail_id = $1
        ORDER BY feature_slug
        LIMIT $2
        "#,
    )
    .bind(trail_id)
    .bind(MAX_TRAIL_FEATURE_ROWS)
    .fetch_all(&mut *transaction)
    .await
    .map_err(|error| internal_error("fetching trail features", error))?;

    let waypoints = sqlx::query_as::<_, WaypointRow>(
        r#"
        SELECT type AS kind, name, elevation_m, ST_X(geom) AS longitude, ST_Y(geom) AS latitude
        FROM waypoints
        WHERE trail_id = $1
        ORDER BY
            CASE type
                WHEN 'peak' THEN 1
                WHEN 'spring' THEN 2
                WHEN 'alpine_hut' THEN 3
                WHEN 'wilderness_hut' THEN 4
                WHEN 'camp_site' THEN 5
                ELSE 10
            END,
            name NULLS LAST,
            id
        LIMIT $2
        "#,
    )
    .bind(trail_id)
    .bind(MAX_TRAIL_WAYPOINT_ROWS)
    .fetch_all(&mut *transaction)
    .await
    .map_err(|error| internal_error("fetching trail waypoints", error))?;

    let diagnostic_rows = sqlx::query_as::<_, StitchDiagnosticRow>(
        r#"
        SELECT kind, ST_AsGeoJSON(geom) AS geometry_geojson
        FROM stitch_diagnostic_points
        WHERE trail_id = $1
        ORDER BY kind, id
        LIMIT $2
        "#,
    )
    .bind(trail_id)
    .bind(MAX_TRAIL_DIAGNOSTIC_ROWS)
    .fetch_all(&mut *transaction)
    .await
    .map_err(|error| internal_error("fetching trail diagnostics", error))?;

    let warnings = sqlx::query_as::<_, StitchWarningRow>(
        r#"
        SELECT message
        FROM stitch_warnings
        WHERE trail_id = $1
        ORDER BY created_at, id
        LIMIT $2
        "#,
    )
    .bind(trail_id)
    .bind(MAX_TRAIL_WARNING_ROWS)
    .fetch_all(&mut *transaction)
    .await
    .map_err(|error| internal_error("fetching trail warnings", error))?;

    let mut region_breadcrumb = if let Some(region_id) = row.region_id {
        fetch_region_paths(&mut *transaction, &[region_id])
            .await
            .map_err(|error| internal_error("fetching region breadcrumb", error))?
            .remove(&region_id)
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    transaction
        .commit()
        .await
        .map_err(|error| internal_error("committing trail detail transaction", error))?;

    let geometry: Option<serde_json::Value> = row
        .geometry_geojson
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .map_err(|error| internal_error("parsing trail geometry", error))?;

    let start_point: Option<serde_json::Value> = row
        .start_point_geojson
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .map_err(|error| internal_error("parsing trail start point", error))?;

    let end_point: Option<serde_json::Value> = row
        .end_point_geojson
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .map_err(|error| internal_error("parsing trail end point", error))?;

    let diagnostics = diagnostic_rows
        .into_iter()
        .map(|row| {
            let geometry: serde_json::Value = serde_json::from_str(&row.geometry_geojson)
                .map_err(|error| internal_error("parsing diagnostic geometry", error))?;
            Ok::<serde_json::Value, actix_web::Error>(serde_json::json!({
                "kind": row.kind,
                "geometry": geometry,
            }))
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;

    prepend_country_to_breadcrumb(&mut region_breadcrumb, row.region_country_code.as_deref());

    let structured_warnings = warnings
        .into_iter()
        .map(|item| TrailWarningPayload {
            kind: classify_warning_kind(&item.message).to_string(),
            message: item.message,
        })
        .collect::<Vec<_>>();

    Ok(cached_json(
        serde_json::json!({
            "id": row.id,
            "data_revision": row.data_revision,
            "osm_relation_id": row.osm_relation_id,
            "last_synced_at": row.last_synced_at,
            "cautions": row.cautions,
            "name": row.name,
            "network": row.network,
            "difficulty": row.difficulty,
            "distance_m": row.distance_m,
            "ascent_m": row.ascent_m,
            "descent_m": row.descent_m,
            "summary": row.summary,
            "hike_type": row.hike_type,
            "estimated_duration_min": row.estimated_duration_min,
            "default_direction_mode": row.default_direction_mode,
            "direction_source": row.direction_source,
            "source_kind": row.source_kind,
            "network_type": row.network_type,
            "route_tag": row.route_tag,
            "route_from": row.route_from,
            "route_to": row.route_to,
            "route_via": row.route_via,
            "website_url": row.website_url,
            "video_url": row.video_url,
            "original_mapper_name": row.original_mapper_name,
            "original_mapper_provider": row.original_mapper_provider,
            "original_mapper_user_id": row.original_mapper_user_id,
            "original_mapper_profile_url": row.original_mapper_profile_url,
            "original_mapped_at": row.original_mapped_at,
            "original_mapper_source": row.original_mapper_source,
            "geometry": geometry,
            "start_point": start_point,
            "end_point": end_point,
            "region_breadcrumb": region_breadcrumb,
            "surfaces": surfaces,
            "features": features.into_iter().map(|item| item.feature_slug).collect::<Vec<_>>(),
            "waypoints": waypoints,
            "stitch_diagnostics": diagnostics,
            "stitch_warnings": structured_warnings,
        }),
        CACHE_TRAILS,
    ))
}

#[get("/trails/{trail_id}/elevation")]
async fn get_elevation(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    query: web::Query<RevisionQuery>,
) -> actix_web::Result<HttpResponse> {
    let trail_id = path.into_inner();

    let row = sqlx::query_as::<_, ElevationRow>(
        r#"
        SELECT
            md5(ST_AsEWKB(trails.geom)::text || trails.updated_at::text) AS data_revision,
            profile.min_elevation_m,
            profile.max_elevation_m,
            profile.profile_points
        FROM trails
        LEFT JOIN elevation_profiles profile ON profile.trail_id = trails.id
        WHERE trails.id = $1
        "#,
    )
    .bind(trail_id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|error| internal_error("fetching elevation profile", error))?;

    let Some(mut row) = row else {
        return Ok(HttpResponse::NotFound()
            .insert_header((header::CACHE_CONTROL, "no-store"))
            .json(serde_json::json!({ "error": "trail not found" })));
    };
    if query
        .revision
        .as_deref()
        .is_some_and(|revision| Some(revision) != row.data_revision.as_deref())
    {
        return Ok(revision_changed_response());
    }
    if row.profile_points.is_none() {
        row.profile_points = Some(serde_json::json!([]));
    }

    Ok(cached_json(row, CACHE_ELEVATION))
}

#[get("/trails/{trail_id}/download")]
async fn download_trail(
    pool: web::Data<PgPool>,
    limits: web::Data<ExportLimits>,
    path: web::Path<Uuid>,
    query: web::Query<DownloadQuery>,
) -> actix_web::Result<HttpResponse> {
    let trail_id = path.into_inner();
    let format = ExportFormat::parse(query.format.as_deref());
    let direction = DownloadDirection::parse(query.direction.as_deref());
    let max_source_points = max_source_export_points(*limits.get_ref(), direction);
    let max_source_points_i64 = i64::try_from(max_source_points)
        .map_err(|error| internal_error("converting export point limit", error))?;
    let max_profile_points_i64 = i64::try_from(limits.max_points)
        .map_err(|error| internal_error("converting profile point limit", error))?;
    let max_source_bytes_i64 = i64::try_from(limits.max_body_bytes)
        .map_err(|error| internal_error("converting export source byte limit", error))?;

    let row = sqlx::query_as::<_, TrailExportRow>(
        r#"
        SELECT
            md5(ST_AsEWKB(source.geom)::text || source.updated_at::text) AS data_revision,
            source.name,
            source.has_geometry,
            source.geometry_point_count,
            source.profile_point_count,
            source.geometry_size_bytes,
            source.profile_size_bytes,
            CASE
                WHEN source.geometry_point_count <= $2
                    AND COALESCE(source.profile_point_count, 0) <= $3
                    AND COALESCE(source.geometry_size_bytes, 0)
                        + COALESCE(source.profile_size_bytes, 0) <= $4
                THEN ST_AsGeoJSON(source.geom)::jsonb
            END AS geometry,
            CASE
                WHEN source.geometry_point_count <= $2
                    AND COALESCE(source.profile_point_count, 0) <= $3
                    AND COALESCE(source.geometry_size_bytes, 0)
                        + COALESCE(source.profile_size_bytes, 0) <= $4
                THEN source.profile_points
            END AS profile_points
        FROM (
            SELECT
                trails.name,
                trails.updated_at,
                trails.geom,
                trails.geom IS NOT NULL AS has_geometry,
                ST_NPoints(trails.geom)::bigint AS geometry_point_count,
                ST_MemSize(trails.geom)::bigint AS geometry_size_bytes,
                elevation_profiles.profile_points,
                jsonb_array_length(elevation_profiles.profile_points)::bigint
                    AS profile_point_count,
                pg_column_size(elevation_profiles.profile_points)::bigint
                    AS profile_size_bytes
            FROM trails
            LEFT JOIN elevation_profiles
                ON elevation_profiles.trail_id = trails.id
            WHERE trails.id = $1
        ) AS source
        "#,
    )
    .bind(trail_id)
    .bind(max_source_points_i64)
    .bind(max_profile_points_i64)
    .bind(max_source_bytes_i64)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|error| internal_error("fetching trail export row", error))?;

    let Some(row) = row else {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({ "error": "trail not found" })));
    };

    if query
        .revision
        .as_deref()
        .is_some_and(|revision| Some(revision) != row.data_revision.as_deref())
    {
        return Ok(revision_changed_response());
    }

    if !row.has_geometry {
        return Ok(
            HttpResponse::NotFound().json(serde_json::json!({ "error": "trail has no geometry" }))
        );
    }

    let geometry_point_count = row
        .geometry_point_count
        .ok_or_else(|| internal_error("checking trail export geometry", "missing point count"))
        .and_then(|count| {
            usize::try_from(count)
                .map_err(|error| internal_error("checking trail export geometry", error))
        })?;
    let profile_point_count = usize::try_from(row.profile_point_count.unwrap_or(0))
        .map_err(|error| internal_error("checking trail export profile", error))?;

    if geometry_point_count > max_source_points || profile_point_count > limits.max_points {
        return Ok(export_too_large_response());
    }
    let geometry_size_bytes = row
        .geometry_size_bytes
        .ok_or_else(|| internal_error("checking trail export geometry", "missing byte size"))?;
    if !export_source_size_within_limit(
        geometry_size_bytes,
        row.profile_size_bytes,
        limits.max_body_bytes,
    ) {
        return Ok(export_too_large_response());
    }

    let geometry = row
        .geometry
        .ok_or_else(|| internal_error("decoding trail export geometry", "missing geometry"))?
        .0;
    let Some(emitted_point_count) = emitted_export_point_count(&geometry, direction) else {
        return Ok(export_too_large_response());
    };
    if emitted_point_count > limits.max_points {
        return Ok(export_too_large_response());
    }

    let trail_name = row.name.unwrap_or_else(|| "Unnamed trail".to_string());
    let file_stem = slugify_filename(&trail_name);
    let profile_points = row
        .profile_points
        .as_ref()
        .map(|points| points.0.as_slice());
    let body = match match format {
        ExportFormat::Gpx => build_gpx_export(
            &trail_name,
            &geometry,
            profile_points,
            direction,
            limits.max_body_bytes,
        ),
        ExportFormat::Kml => build_kml_export(
            &trail_name,
            &geometry,
            profile_points,
            direction,
            limits.max_body_bytes,
        ),
        ExportFormat::GeoJson => build_geojson_export(
            &trail_name,
            &geometry,
            profile_points,
            direction,
            limits.max_body_bytes,
        ),
    } {
        Ok(body) => body,
        Err(ExportBuildError::BodyTooLarge) => return Ok(export_too_large_response()),
        Err(error) => return Err(internal_error("building trail export", error)),
    };

    Ok(HttpResponse::Ok()
        .insert_header((header::CACHE_CONTROL, CACHE_TRAILS))
        .insert_header((header::CONTENT_TYPE, format.content_type()))
        .insert_header((
            header::CONTENT_DISPOSITION,
            format!(
                "attachment; filename=\"{}-{}.{}\"",
                file_stem,
                match direction {
                    DownloadDirection::Forward => "forward",
                    DownloadDirection::Backward => "backward",
                    DownloadDirection::Both => "both",
                },
                format.extension()
            ),
        ))
        .body(body))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "hiking_api=info,actix_web=info,sqlx=warn".into()),
        )
        .init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let config = ApiConfig::from_env().expect("invalid API server configuration");
    let pool = hiking_db::create_pool(&database_url)
        .await
        .expect("failed to connect to database");
    let request_policy = web::Data::new(RequestPolicy {
        permits: Arc::new(Semaphore::new(config.max_in_flight_requests)),
        readiness_permits: Arc::new(Semaphore::new(1)),
        request_timeout: config.request_timeout,
        queue_timeout: config.queue_timeout,
        readiness_timeout: config.readiness_timeout,
    });
    let export_limits = web::Data::new(config.export_limits);

    tracing::info!(
        bind_addr = %config.bind_addr,
        workers = config.workers,
        max_in_flight_requests = config.max_in_flight_requests,
        readiness_timeout_ms = config.readiness_timeout.as_millis(),
        export_max_points = config.export_limits.max_points,
        export_max_body_bytes = config.export_limits.max_body_bytes,
        "starting hiking-api"
    );

    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(request_policy.clone())
            .app_data(export_limits.clone())
            .wrap(middleware::from_fn(enforce_request_policy))
            .wrap(middleware::Compress::default())
            .wrap(
                middleware::Logger::new(r#"%a "%r" %s %b %Dms"#)
                    .exclude("/health")
                    .exclude("/ready"),
            )
            .service(health)
            .service(ready)
            .service(search)
            .service(discovery::discover_trails)
            .service(list_trails_in_region)
            .service(get_trail)
            .service(get_elevation)
            .service(download_trail)
    })
    .workers(config.workers)
    .max_connections(config.max_connections)
    .backlog(config.backlog)
    .keep_alive(KeepAlive::Timeout(config.keep_alive))
    .client_request_timeout(config.client_request_timeout)
    .client_disconnect_timeout(config.client_disconnect_timeout)
    .shutdown_timeout(config.shutdown_timeout_seconds)
    .bind(&config.bind_addr)?;

    server.run().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn india_admin_levels_use_local_search_labels() {
        assert_eq!(admin_level_label(Some("IN"), Some(4)), Some("State"));
        assert_eq!(admin_level_label(Some("IN"), Some(5)), Some("District"));
        assert_eq!(admin_level_label(Some("IN"), Some(6)), Some("Subdistrict"));
    }

    #[test]
    fn public_query_bounds_are_enforced() {
        assert_eq!(
            list_offset(&TrailListQuery {
                limit: None,
                offset: Some(-1),
            }),
            0
        );
        assert_eq!(
            list_offset(&TrailListQuery {
                limit: None,
                offset: Some(i64::MAX),
            }),
            MAX_TRAIL_LIST_OFFSET
        );
        assert!(!search_term_is_eligible("a"));
        assert!(!search_term_is_eligible("in"));
        assert!(!search_term_is_eligible("山道"));
        assert!(search_term_is_eligible("man"));
        assert!(search_term_is_eligible("山道口"));
    }

    #[test]
    fn trail_detail_child_collection_limits_are_stable() {
        assert_eq!(MAX_TRAIL_SURFACE_ROWS, 256);
        assert_eq!(MAX_TRAIL_FEATURE_ROWS, 256);
        assert_eq!(MAX_TRAIL_WAYPOINT_ROWS, 2_000);
        assert_eq!(MAX_TRAIL_DIAGNOSTIC_ROWS, 2_000);
        assert_eq!(MAX_TRAIL_WARNING_ROWS, 500);
    }

    #[test]
    fn search_patterns_escape_sql_wildcards_literally() {
        assert_eq!(escape_ilike_pattern(r"a\b%c_d"), r"a\\b\%c\_d");
        assert_eq!(escape_ilike_pattern("山道口"), "山道口");
    }

    #[test]
    fn readiness_guard_allows_only_one_non_waiting_check() {
        let policy = RequestPolicy {
            permits: Arc::new(Semaphore::new(1)),
            readiness_permits: Arc::new(Semaphore::new(1)),
            request_timeout: Duration::from_secs(1),
            queue_timeout: Duration::from_secs(1),
            readiness_timeout: Duration::from_secs(1),
        };

        let first = policy.try_acquire_readiness().expect("first permit");
        assert!(policy.try_acquire_readiness().is_none());
        drop(first);
        assert!(policy.try_acquire_readiness().is_some());
    }

    #[test]
    fn probe_routes_bypass_the_normal_request_queue() {
        assert!(bypass_normal_request_policy("/health"));
        assert!(bypass_normal_request_policy("/ready"));
        assert!(!bypass_normal_request_policy("/search"));
    }

    #[test]
    fn elevation_cursor_interpolates_monotonic_positions_in_one_pass() {
        let profile = vec![
            ElevationPoint {
                pos_m: 0.0,
                ele_m: 100.0,
            },
            ElevationPoint {
                pos_m: 50.0,
                ele_m: 200.0,
            },
            ElevationPoint {
                pos_m: 100.0,
                ele_m: 150.0,
            },
        ];
        let mut cursor = ElevationCursor::new(&profile);

        let elevations = [0.0, 25.0, 50.0, 75.0, 100.0, 120.0]
            .into_iter()
            .map(|position| cursor.elevation_at(position).expect("elevation"))
            .collect::<Vec<_>>();

        assert_eq!(elevations, vec![100.0, 150.0, 200.0, 175.0, 150.0, 150.0]);
    }

    #[test]
    fn bidirectional_export_reuses_geometry_and_reverses_line_order() {
        let geometry = TrailGeometry::MultiLineString {
            coordinates: vec![
                vec![[77.0, 32.0], [77.001, 32.0]],
                vec![[77.001, 32.0], [77.002, 32.0]],
            ],
        };
        let line_strings = geometry.line_strings();
        let total_distance = total_route_distance_m(&line_strings);
        let profile = vec![
            ElevationPoint {
                pos_m: 0.0,
                ele_m: 1_000.0,
            },
            ElevationPoint {
                pos_m: total_distance,
                ele_m: 1_200.0,
            },
        ];

        let body = build_geojson_export(
            "Test trail",
            &geometry,
            Some(&profile),
            DownloadDirection::Both,
            1024 * 1024,
        )
        .expect("build export");
        let export: serde_json::Value = serde_json::from_slice(&body).expect("valid GeoJSON");
        let features = export["features"].as_array().expect("features");

        assert_eq!(features.len(), 2);
        assert_eq!(features[0]["properties"]["direction"], "forward");
        assert_eq!(features[1]["properties"]["direction"], "backward");
        assert_eq!(
            features[0]["geometry"]["coordinates"][0][0][0].as_f64(),
            Some(77.0)
        );
        assert_eq!(
            features[1]["geometry"]["coordinates"][0][0][0].as_f64(),
            Some(77.002)
        );
        assert_eq!(
            features[0]["geometry"]["coordinates"][0][0][2].as_f64(),
            Some(1_000.0)
        );
        assert_eq!(
            features[1]["geometry"]["coordinates"][0][0][2].as_f64(),
            Some(1_200.0)
        );
        assert_eq!(
            features[1]["geometry"]["coordinates"][1][1][2].as_f64(),
            Some(1_000.0)
        );
    }

    #[test]
    fn export_limits_count_nested_and_bidirectional_points() {
        let geometry = TrailGeometry::GeometryCollection {
            geometries: vec![
                TrailGeometry::LineString {
                    coordinates: vec![[77.0, 32.0], [77.1, 32.1]],
                },
                TrailGeometry::MultiLineString {
                    coordinates: vec![vec![[78.0, 33.0]], vec![[79.0, 34.0], [79.1, 34.1]]],
                },
            ],
        };
        let limits = ExportLimits {
            max_points: 10,
            max_body_bytes: 1024,
        };

        assert_eq!(geometry.point_count(), Some(5));
        assert_eq!(max_source_export_points(limits, DownloadDirection::Both), 5);
        assert_eq!(
            emitted_export_point_count(&geometry, DownloadDirection::Both),
            Some(10)
        );
    }

    #[test]
    fn export_source_size_limit_rejects_invalid_and_combined_oversized_inputs() {
        assert!(export_source_size_within_limit(600, Some(400), 1_000));
        assert!(!export_source_size_within_limit(601, Some(400), 1_000));
        assert!(!export_source_size_within_limit(-1, None, 1_000));
    }

    #[test]
    fn bounded_export_body_never_exceeds_its_limit() {
        let mut body = BoundedBody::new(4);
        body.append_str("1234").expect("exactly at limit");
        assert_eq!(body.len(), 4);
        assert!(matches!(
            body.append_str("5"),
            Err(ExportBuildError::BodyTooLarge)
        ));
        assert_eq!(body.len(), 4);
    }

    #[test]
    fn every_export_format_rejects_an_oversized_body() {
        let geometry = TrailGeometry::LineString {
            coordinates: vec![[77.0, 32.0], [77.1, 32.1]],
        };
        let results = [
            build_geojson_export("Test trail", &geometry, None, DownloadDirection::Forward, 1),
            build_gpx_export("Test trail", &geometry, None, DownloadDirection::Forward, 1),
            build_kml_export("Test trail", &geometry, None, DownloadDirection::Forward, 1),
        ];

        assert!(results
            .into_iter()
            .all(|result| matches!(result, Err(ExportBuildError::BodyTooLarge))));
    }

    #[test]
    fn oversized_export_response_is_not_cached() {
        let response = export_too_large_response();

        assert_eq!(
            response.status(),
            actix_web::http::StatusCode::PAYLOAD_TOO_LARGE
        );
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL),
            Some(&header::HeaderValue::from_static("no-store"))
        );
    }
}
