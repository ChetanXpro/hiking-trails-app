use actix_web::{get, http::header, web, App, HttpResponse, HttpServer};
use chrono::{DateTime, Utc};
use hiking_db::models::ElevationPoint;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use std::collections::HashMap;
use std::fmt::Display;
use tokio::try_join;
use uuid::Uuid;

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
    id: Uuid,
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
    min_elevation_m: Option<f64>,
    max_elevation_m: Option<f64>,
    profile_points: serde_json::Value,
}

#[derive(Debug, FromRow)]
struct TrailExportRow {
    name: Option<String>,
    geometry_geojson: Option<String>,
    profile_points: Option<sqlx::types::Json<Vec<ElevationPoint>>>,
}

#[derive(Debug, Deserialize)]
struct DownloadQuery {
    format: Option<String>,
    direction: Option<String>,
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

#[derive(Debug, Clone)]
struct ExportTrack {
    label: &'static str,
    line_strings: Vec<Vec<ExportPoint>>,
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
}

impl TrailGeometry {
    fn reversed(&self) -> Self {
        match self {
            Self::LineString { coordinates } => Self::LineString {
                coordinates: coordinates.iter().copied().rev().collect(),
            },
            Self::MultiLineString { coordinates } => Self::MultiLineString {
                coordinates: coordinates
                    .iter()
                    .rev()
                    .map(|line| line.iter().copied().rev().collect())
                    .collect(),
            },
            Self::GeometryCollection { geometries } => Self::GeometryCollection {
                geometries: geometries.iter().map(TrailGeometry::reversed).collect(),
            },
        }
    }

    fn line_strings(&self) -> Vec<Vec<[f64; 2]>> {
        match self {
            Self::LineString { coordinates } => vec![coordinates.clone()],
            Self::MultiLineString { coordinates } => coordinates.clone(),
            Self::GeometryCollection { geometries } => geometries
                .iter()
                .flat_map(TrailGeometry::line_strings)
                .collect(),
        }
    }
}

fn internal_error(context: &'static str, error: impl Display) -> actix_web::Error {
    tracing::error!(context, error = %error, "request failed");
    actix_web::error::ErrorInternalServerError("internal server error")
}

fn list_limit(query: &TrailListQuery) -> i64 {
    query.limit.unwrap_or(250).clamp(1, 1_000)
}

fn list_offset(query: &TrailListQuery) -> i64 {
    query.offset.unwrap_or(0).max(0)
}

fn search_limit(query: &SearchQuery) -> i64 {
    query.limit.unwrap_or(12).clamp(1, 20)
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

async fn fetch_region_paths(
    pool: &PgPool,
    region_ids: &[Uuid],
) -> Result<HashMap<Uuid, Vec<String>>, sqlx::Error> {
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
    .fetch_all(pool)
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

fn total_route_distance_m(line_strings: &[Vec<[f64; 2]>]) -> f64 {
    line_strings
        .iter()
        .flat_map(|line| line.windows(2))
        .map(|pair| approx_distance_m(pair[0], pair[1]))
        .sum()
}

fn reverse_profile_points(
    profile_points: &[ElevationPoint],
    total_distance_m: f64,
) -> Vec<ElevationPoint> {
    profile_points
        .iter()
        .rev()
        .map(|point| ElevationPoint {
            pos_m: (total_distance_m - point.pos_m).max(0.0),
            ele_m: point.ele_m,
        })
        .collect()
}

fn elevation_at_position(profile_points: &[ElevationPoint], pos_m: f64) -> Option<f64> {
    let first = profile_points.first()?;
    if pos_m <= first.pos_m {
        return Some(first.ele_m);
    }

    let last = profile_points.last()?;
    if pos_m >= last.pos_m {
        return Some(last.ele_m);
    }

    for window in profile_points.windows(2) {
        let left = &window[0];
        let right = &window[1];
        if pos_m > right.pos_m {
            continue;
        }

        let span = (right.pos_m - left.pos_m).max(f64::EPSILON);
        let fraction = ((pos_m - left.pos_m) / span).clamp(0.0, 1.0);
        return Some(left.ele_m + (right.ele_m - left.ele_m) * fraction);
    }

    Some(last.ele_m)
}

fn build_export_track(
    label: &'static str,
    geometry: &TrailGeometry,
    profile_points: Option<&[ElevationPoint]>,
) -> ExportTrack {
    let raw_line_strings = geometry.line_strings();
    let mut cumulative_m = 0.0;
    let line_strings = raw_line_strings
        .into_iter()
        .map(|line| {
            let mut last_coord: Option<[f64; 2]> = None;
            line.into_iter()
                .map(|coord| {
                    if let Some(previous) = last_coord {
                        cumulative_m += approx_distance_m(previous, coord);
                    }
                    last_coord = Some(coord);
                    ExportPoint {
                        lon: coord[0],
                        lat: coord[1],
                        ele: profile_points
                            .and_then(|points| elevation_at_position(points, cumulative_m)),
                    }
                })
                .collect::<Vec<_>>()
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();

    ExportTrack {
        label,
        line_strings,
    }
}

fn export_tracks(
    geometry: &TrailGeometry,
    profile_points: Option<&[ElevationPoint]>,
    direction: DownloadDirection,
) -> Vec<ExportTrack> {
    let total_distance_m = total_route_distance_m(&geometry.line_strings());
    let backward_profile =
        profile_points.map(|points| reverse_profile_points(points, total_distance_m));

    match direction {
        DownloadDirection::Forward => vec![build_export_track("forward", geometry, profile_points)],
        DownloadDirection::Backward => vec![build_export_track(
            "backward",
            &geometry.reversed(),
            backward_profile.as_deref(),
        )],
        DownloadDirection::Both => vec![
            build_export_track("forward", geometry, profile_points),
            build_export_track(
                "backward",
                &geometry.reversed(),
                backward_profile.as_deref(),
            ),
        ],
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

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn build_geojson_export(
    name: &str,
    geometry: &TrailGeometry,
    profile_points: Option<&[ElevationPoint]>,
    direction: DownloadDirection,
) -> String {
    let features = export_tracks(geometry, profile_points, direction)
        .into_iter()
        .map(|track| {
            let geometry = if track.line_strings.len() == 1 {
                serde_json::json!({
                    "type": "LineString",
                    "coordinates": track.line_strings[0]
                        .iter()
                        .map(|point| serde_json::json!([point.lon, point.lat, point.ele.unwrap_or(0.0)]))
                        .collect::<Vec<_>>(),
                })
            } else {
                serde_json::json!({
                    "type": "MultiLineString",
                    "coordinates": track.line_strings
                        .iter()
                        .map(|line| {
                            line.iter()
                                .map(|point| serde_json::json!([point.lon, point.lat, point.ele.unwrap_or(0.0)]))
                                .collect::<Vec<_>>()
                        })
                        .collect::<Vec<_>>(),
                })
            };

            serde_json::json!({
                "type": "Feature",
                "properties": {
                    "name": name,
                    "direction": track.label,
                },
                "geometry": geometry,
            })
        })
        .collect::<Vec<_>>();

    serde_json::to_string_pretty(&serde_json::json!({
        "type": "FeatureCollection",
        "features": features,
    }))
    .unwrap_or_else(|_| "{\"type\":\"FeatureCollection\",\"features\":[]}".to_string())
}

fn build_gpx_export(
    name: &str,
    geometry: &TrailGeometry,
    profile_points: Option<&[ElevationPoint]>,
    direction: DownloadDirection,
) -> String {
    let escaped_name = escape_xml(name);
    let tracks = export_tracks(geometry, profile_points, direction)
        .into_iter()
        .map(|track| {
            let segments = track
                .line_strings
                .into_iter()
                .filter(|line| !line.is_empty())
                .map(|line| {
                    let points = line
                        .into_iter()
                        .map(|point| {
                            let ele = point
                                .ele
                                .map(|value| format!("<ele>{value:.1}</ele>"))
                                .unwrap_or_default();
                            format!(
                                r#"<trkpt lat="{lat:.7}" lon="{lon:.7}">{ele}</trkpt>"#,
                                lat = point.lat,
                                lon = point.lon
                            )
                        })
                        .collect::<String>();
                    format!("<trkseg>{points}</trkseg>")
                })
                .collect::<String>();

            format!(
                "<trk><name>{escaped_name} ({label})</name>{segments}</trk>",
                label = escape_xml(track.label)
            )
        })
        .collect::<String>();

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="hiking-trails-app" xmlns="http://www.topografix.com/GPX/1/1">
  {tracks}
</gpx>
"#
    )
}

fn build_kml_export(
    name: &str,
    geometry: &TrailGeometry,
    profile_points: Option<&[ElevationPoint]>,
    direction: DownloadDirection,
) -> String {
    let escaped_name = escape_xml(name);
    let placemarks = export_tracks(geometry, profile_points, direction)
        .into_iter()
        .map(|track| {
            let line_strings = track
                .line_strings
                .into_iter()
                .filter(|line| !line.is_empty())
                .map(|line| {
                    let coords = line
                        .into_iter()
                        .map(|point| {
                            format!(
                                "{:.7},{:.7},{:.1}",
                                point.lon,
                                point.lat,
                                point.ele.unwrap_or(0.0)
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    format!(
                        "<LineString><tessellate>1</tessellate><coordinates>{coords}</coordinates></LineString>"
                    )
                })
                .collect::<String>();

            format!(
                "<Placemark><name>{escaped_name} ({label})</name><MultiGeometry>{line_strings}</MultiGeometry></Placemark>",
                label = escape_xml(track.label)
            )
        })
        .collect::<String>();

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<kml xmlns="http://www.opengis.net/kml/2.2">
  <Document>
    {placemarks}
  </Document>
</kml>
"#
    )
}

#[get("/health")]
async fn health() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({ "status": "ok" }))
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
        WHERE region_id = $1
        ORDER BY name
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

    Ok(HttpResponse::Ok().json(trails))
}

#[get("/search")]
async fn search(
    pool: web::Data<PgPool>,
    query: web::Query<SearchQuery>,
) -> actix_web::Result<HttpResponse> {
    let query = query.into_inner();
    let raw_term = query.q.clone().unwrap_or_default();
    let term = raw_term.trim();

    if term.is_empty() {
        return Ok(HttpResponse::Ok().json(SearchResponse {
            routes: Vec::new(),
            places: Vec::new(),
        }));
    }

    let route_rows = sqlx::query_as::<_, SearchTrailRow>(
        r#"
        SELECT
            id,
            name,
            network,
            distance_m,
            region_id,
            region_country_code,
            tags->>'from' AS route_from,
            tags->>'to' AS route_to,
            tags->>'via' AS route_via,
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
                raw.tags,
                trails.summary,
                (
                    CASE
                        WHEN lower(COALESCE(trails.name, '')) = lower($1) THEN 1000
                        WHEN lower(COALESCE(trails.name, '')) LIKE lower($1) || '%' THEN 850
                        WHEN COALESCE(trails.name, '') ILIKE '%' || $1 || '%' THEN 700
                        WHEN COALESCE(raw.tags->>'from', '') ILIKE '%' || $1 || '%' THEN 560
                        WHEN COALESCE(raw.tags->>'to', '') ILIKE '%' || $1 || '%' THEN 560
                        WHEN COALESCE(raw.tags->>'via', '') ILIKE '%' || $1 || '%' THEN 540
                        WHEN COALESCE(trails.summary, '') ILIKE '%' || $1 || '%' THEN 480
                        ELSE 0
                    END
                    + (
                        GREATEST(
                            similarity(COALESCE(trails.name, ''), $1),
                            similarity(COALESCE(raw.tags->>'from', ''), $1),
                            similarity(COALESCE(raw.tags->>'to', ''), $1),
                            similarity(COALESCE(raw.tags->>'via', ''), $1),
                            similarity(COALESCE(trails.summary, ''), $1)
                        ) * 100
                    )
                ) AS _score
            FROM trails
            LEFT JOIN raw_relations raw ON raw.osm_id = trails.osm_relation_id
            LEFT JOIN regions ON regions.id = trails.region_id
            WHERE
                COALESCE(trails.name, '') ILIKE '%' || $1 || '%'
                OR COALESCE(trails.summary, '') ILIKE '%' || $1 || '%'
                OR COALESCE(raw.tags->>'from', '') ILIKE '%' || $1 || '%'
                OR COALESCE(raw.tags->>'to', '') ILIKE '%' || $1 || '%'
                OR COALESCE(raw.tags->>'via', '') ILIKE '%' || $1 || '%'
                OR similarity(COALESCE(trails.name, ''), $1) > 0.2
        ) matches
        ORDER BY _score DESC, name NULLS LAST
        LIMIT $2
        "#,
    )
    .bind(term)
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
                    WHEN lower(name) LIKE lower($1) || '%' THEN 920
                    WHEN name ILIKE '%' || $1 || '%' THEN 760
                    ELSE 0
                END
                + similarity(name, $1) * 100
            ) AS _score
        FROM regions
        WHERE
            name ILIKE '%' || $1 || '%'
            OR similarity(name, $1) > 0.2
        ORDER BY _score DESC, name
        LIMIT $2
        "#,
    )
    .bind(term)
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
                    WHEN lower(name) LIKE lower($1) || '%' THEN 880
                    WHEN name ILIKE '%' || $1 || '%' THEN 720
                    ELSE 0
                END
                + similarity(name, $1) * 100
            ) AS _score
        FROM places
        WHERE
            name ILIKE '%' || $1 || '%'
            OR similarity(name, $1) > 0.2
        ORDER BY _score DESC, name
        LIMIT $2
        "#,
    )
    .bind(term)
    .bind(search_limit(&query))
    .fetch_all(pool.get_ref())
    .await
    .map_err(|error| internal_error("searching places", error))?;

    let country_bbox_rows = sqlx::query_as::<_, CountryBoundsRow>(
        r#"
        SELECT
            country_code,
            MIN(ST_XMin(ST_Envelope(boundary))) AS min_lon,
            MIN(ST_YMin(ST_Envelope(boundary))) AS min_lat,
            MAX(ST_XMax(ST_Envelope(boundary))) AS max_lon,
            MAX(ST_YMax(ST_Envelope(boundary))) AS max_lat
        FROM regions
        WHERE country_code IS NOT NULL
        GROUP BY country_code
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

    Ok(HttpResponse::Ok().json(SearchResponse { routes, places }))
}

#[get("/trails/{trail_id}")]
async fn get_trail(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> actix_web::Result<HttpResponse> {
    let trail_id = path.into_inner();

    let row = sqlx::query_as::<_, TrailDetailRow>(
        r#"
        SELECT
            trails.id,
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
            raw.tags->>'network:type' AS network_type,
            raw.tags->>'tag' AS route_tag,
            raw.tags->>'from' AS route_from,
            raw.tags->>'to' AS route_to,
            raw.tags->>'via' AS route_via,
            raw.tags->>'website' AS website_url,
            raw.tags->>'video' AS video_url,
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
        LEFT JOIN raw_relations raw ON raw.osm_id = trails.osm_relation_id
        LEFT JOIN regions ON regions.id = trails.region_id
        WHERE trails.id = $1
        "#,
    )
    .bind(trail_id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|error| internal_error("fetching trail detail row", error))?;

    let Some(row) = row else {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({ "error": "trail not found" })));
    };

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

    let pool_ref = pool.get_ref();
    let surfaces_fut = sqlx::query_as::<_, SurfaceBreakdownRow>(
        r#"
        SELECT surface_type, distance_m, pct
        FROM trail_surface_breakdown
        WHERE trail_id = $1
        ORDER BY distance_m DESC, surface_type
        "#,
    )
    .bind(trail_id)
    .fetch_all(pool_ref);

    let features_fut = sqlx::query_as::<_, FeatureRow>(
        r#"
        SELECT feature_slug
        FROM trail_features
        WHERE trail_id = $1
        ORDER BY feature_slug
        "#,
    )
    .bind(trail_id)
    .fetch_all(pool_ref);

    let waypoints_fut = sqlx::query_as::<_, WaypointRow>(
        r#"
        SELECT type AS kind, name, elevation_m
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
            name NULLS LAST
        "#,
    )
    .bind(trail_id)
    .fetch_all(pool_ref);

    let diagnostic_rows_fut = sqlx::query_as::<_, StitchDiagnosticRow>(
        r#"
        SELECT kind, ST_AsGeoJSON(geom) AS geometry_geojson
        FROM stitch_diagnostic_points
        WHERE trail_id = $1
        ORDER BY kind
        "#,
    )
    .bind(trail_id)
    .fetch_all(pool_ref);

    let warnings_fut = sqlx::query_as::<_, StitchWarningRow>(
        r#"
        SELECT message
        FROM stitch_warnings
        WHERE trail_id = $1
        ORDER BY created_at, id
        "#,
    )
    .bind(trail_id)
    .fetch_all(pool_ref);

    let (surfaces, features, waypoints, diagnostic_rows, warnings) = try_join!(
        surfaces_fut,
        features_fut,
        waypoints_fut,
        diagnostic_rows_fut,
        warnings_fut
    )
    .map_err(|error| internal_error("fetching trail detail collections", error))?;

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

    let mut region_breadcrumb = if let Some(region_id) = row.region_id {
        fetch_region_paths(pool.get_ref(), &[region_id])
            .await
            .map_err(|error| internal_error("fetching region breadcrumb", error))?
            .remove(&region_id)
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    prepend_country_to_breadcrumb(&mut region_breadcrumb, row.region_country_code.as_deref());

    let structured_warnings = warnings
        .into_iter()
        .map(|item| TrailWarningPayload {
            kind: classify_warning_kind(&item.message).to_string(),
            message: item.message,
        })
        .collect::<Vec<_>>();

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "id": row.id,
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
    })))
}

#[get("/trails/{trail_id}/elevation")]
async fn get_elevation(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> actix_web::Result<HttpResponse> {
    let trail_id = path.into_inner();

    let row = sqlx::query_as::<_, ElevationRow>(
        r#"
        SELECT min_elevation_m, max_elevation_m, profile_points
        FROM elevation_profiles
        WHERE trail_id = $1
        "#,
    )
    .bind(trail_id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|error| internal_error("fetching elevation profile", error))?;

    let row = row.unwrap_or(ElevationRow {
        min_elevation_m: None,
        max_elevation_m: None,
        profile_points: serde_json::json!([]),
    });

    Ok(HttpResponse::Ok().json(row))
}

#[get("/trails/{trail_id}/download")]
async fn download_trail(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    query: web::Query<DownloadQuery>,
) -> actix_web::Result<HttpResponse> {
    let trail_id = path.into_inner();
    let format = ExportFormat::parse(query.format.as_deref());
    let direction = DownloadDirection::parse(query.direction.as_deref());

    let row = sqlx::query_as::<_, TrailExportRow>(
        r#"
        SELECT
            name,
            ST_AsGeoJSON(trails.geom) AS geometry_geojson,
            elevation_profiles.profile_points
        FROM trails
        LEFT JOIN elevation_profiles ON elevation_profiles.trail_id = trails.id
        WHERE id = $1
        "#,
    )
    .bind(trail_id)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|error| internal_error("fetching trail export row", error))?;

    let Some(row) = row else {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({ "error": "trail not found" })));
    };

    let Some(geometry_geojson) = row.geometry_geojson else {
        return Ok(
            HttpResponse::NotFound().json(serde_json::json!({ "error": "trail has no geometry" }))
        );
    };

    let geometry: TrailGeometry = serde_json::from_str(&geometry_geojson)
        .map_err(|error| internal_error("parsing export geometry", error))?;

    let trail_name = row.name.unwrap_or_else(|| "Unnamed trail".to_string());
    let file_stem = slugify_filename(&trail_name);
    let profile_points = row
        .profile_points
        .as_ref()
        .map(|points| points.0.as_slice());
    let body = match format {
        ExportFormat::Gpx => build_gpx_export(&trail_name, &geometry, profile_points, direction),
        ExportFormat::Kml => build_kml_export(&trail_name, &geometry, profile_points, direction),
        ExportFormat::GeoJson => {
            build_geojson_export(&trail_name, &geometry, profile_points, direction)
        }
    };

    Ok(HttpResponse::Ok()
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
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = hiking_db::create_pool(&database_url)
        .await
        .expect("failed to connect to database");

    tracing::info!("starting hiking-api on 0.0.0.0:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .service(health)
            .service(search)
            .service(list_trails_in_region)
            .service(get_trail)
            .service(get_elevation)
            .service(download_trail)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
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
}
