use actix_web::{get, http::header, web, App, HttpResponse, HttpServer};
use hiking_db::models::ElevationPoint;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
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

#[derive(Debug, Serialize, FromRow)]
struct RegionBreadcrumbRow {
    name: String,
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

#[derive(Debug, Serialize)]
struct TrailWarningPayload {
    kind: String,
    message: String,
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

#[get("/trails/{trail_id}")]
async fn get_trail(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> actix_web::Result<HttpResponse> {
    let trail_id = path.into_inner();

    let row = sqlx::query_as::<_, TrailDetailRow>(
        r#"
        SELECT
            id,
            region_id,
            name,
            network,
            difficulty,
            distance_m,
            ascent_m,
            descent_m,
            summary,
            hike_type,
            estimated_duration_min,
            default_direction_mode,
            direction_source,
            source_kind,
            raw.tags->>'network:type' AS network_type,
            raw.tags->>'tag' AS route_tag,
            raw.tags->>'from' AS route_from,
            raw.tags->>'to' AS route_to,
            raw.tags->>'via' AS route_via,
            raw.tags->>'website' AS website_url,
            raw.tags->>'video' AS video_url,
            ST_AsGeoJSON(trails.geom) AS geometry_geojson,
            ST_AsGeoJSON(start_point) AS start_point_geojson,
            ST_AsGeoJSON(end_point) AS end_point_geojson
        FROM trails
        LEFT JOIN raw_relations raw ON raw.osm_id = trails.osm_relation_id
        WHERE id = $1
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

    let region_breadcrumb = if let Some(region_id) = row.region_id {
        sqlx::query_as::<_, RegionBreadcrumbRow>(
            r#"
            WITH RECURSIVE chain AS (
                SELECT id, name, parent_region_id, 0 AS depth
                FROM regions
                WHERE id = $1

                UNION ALL

                SELECT regions.id, regions.name, regions.parent_region_id, chain.depth + 1
                FROM regions
                JOIN chain ON chain.parent_region_id = regions.id
            )
            SELECT name
            FROM chain
            ORDER BY depth DESC
            "#,
        )
        .bind(region_id)
        .fetch_all(pool.get_ref())
        .await
        .map_err(|error| internal_error("fetching region breadcrumb", error))?
    } else {
        Vec::new()
    };

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
        "geometry": geometry,
        "start_point": start_point,
        "end_point": end_point,
        "region_breadcrumb": region_breadcrumb.into_iter().map(|item| item.name).collect::<Vec<_>>(),
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
            .service(list_trails_in_region)
            .service(get_trail)
            .service(get_elevation)
            .service(download_trail)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
