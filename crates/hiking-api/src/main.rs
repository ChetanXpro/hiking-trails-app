use actix_web::{get, web, App, HttpResponse, HttpServer};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Serialize)]
struct TrailSummary {
    id: Uuid,
    name: Option<String>,
    network: Option<String>,
    difficulty: Option<String>,
    distance_m: Option<f64>,
    ascent_m: Option<f64>,
    descent_m: Option<f64>,
}

#[get("/health")]
async fn health() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({ "status": "ok" }))
}

#[get("/regions/{region_id}/trails")]
async fn list_trails_in_region(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> actix_web::Result<HttpResponse> {
    let region_id = path.into_inner();

    let trails = sqlx::query_as!(
        TrailSummary,
        r#"
        SELECT id, name, network, difficulty, distance_m, ascent_m, descent_m
        FROM trails
        WHERE region_id = $1
        ORDER BY name
        "#,
        region_id
    )
    .fetch_all(pool.get_ref())
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(trails))
}

#[derive(Serialize)]
struct TrailDetailRow {
    id: Uuid,
    name: Option<String>,
    network: Option<String>,
    difficulty: Option<String>,
    distance_m: Option<f64>,
    ascent_m: Option<f64>,
    descent_m: Option<f64>,
    geometry_geojson: Option<String>,
    warnings: serde_json::Value,
    diagnostic_points: serde_json::Value,
}

#[get("/trails/{trail_id}")]
async fn get_trail(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> actix_web::Result<HttpResponse> {
    let trail_id = path.into_inner();

    let row = sqlx::query_as!(
        TrailDetailRow,
        r#"
        SELECT t.id, t.name, t.network, t.difficulty, t.distance_m, t.ascent_m, t.descent_m,
               ST_AsGeoJSON(t.geom) AS geometry_geojson,
               COALESCE(
                   (SELECT jsonb_agg(w.message ORDER BY w.created_at)
                    FROM stitch_warnings w WHERE w.trail_id = t.id),
                   '[]'::jsonb
               ) as "warnings!",
               COALESCE(
                   (SELECT jsonb_agg(jsonb_build_object(
                       'type', 'Feature',
                       'geometry', ST_AsGeoJSON(p.geom)::jsonb,
                       'properties', jsonb_build_object('kind', p.kind)
                   ))
                    FROM stitch_diagnostic_points p WHERE p.trail_id = t.id),
                   '[]'::jsonb
               ) as "diagnostic_points!"
        FROM trails t
        WHERE t.id = $1
        "#,
        trail_id
    )
    .fetch_optional(pool.get_ref())
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    let Some(row) = row else {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({ "error": "trail not found" })));
    };

    let geometry: Option<serde_json::Value> = row
        .geometry_geojson
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "id": row.id,
        "name": row.name,
        "network": row.network,
        "difficulty": row.difficulty,
        "distance_m": row.distance_m,
        "ascent_m": row.ascent_m,
        "descent_m": row.descent_m,
        "geometry": geometry,
        "warnings": row.warnings,
        "diagnostic_points": {
            "type": "FeatureCollection",
            "features": row.diagnostic_points,
        },
    })))
}

#[derive(Serialize)]
struct ElevationRow {
    min_elevation_m: Option<f64>,
    max_elevation_m: Option<f64>,
    profile_points: serde_json::Value,
}

#[get("/trails/{trail_id}/elevation")]
async fn get_elevation(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> actix_web::Result<HttpResponse> {
    let trail_id = path.into_inner();

    let row = sqlx::query_as!(
        ElevationRow,
        r#"
        SELECT min_elevation_m, max_elevation_m, profile_points
        FROM elevation_profiles
        WHERE trail_id = $1
        "#,
        trail_id
    )
    .fetch_optional(pool.get_ref())
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    let Some(row) = row else {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({ "error": "no elevation profile for this trail" })));
    };

    Ok(HttpResponse::Ok().json(row))
}

#[derive(Serialize)]
struct GeojsonRow {
    geojson: serde_json::Value,
}

#[get("/waypoints")]
async fn list_waypoints(pool: web::Data<PgPool>) -> actix_web::Result<HttpResponse> {
    let row = sqlx::query_as!(
        GeojsonRow,
        r#"
        SELECT jsonb_build_object(
            'type', 'FeatureCollection',
            'features', COALESCE(jsonb_agg(
                jsonb_build_object(
                    'type', 'Feature',
                    'geometry', ST_AsGeoJSON(geom)::jsonb,
                    'properties', jsonb_build_object('id', id, 'type', type, 'name', name)
                )
            ), '[]'::jsonb)
        ) as "geojson!"
        FROM waypoints
        "#
    )
    .fetch_one(pool.get_ref())
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(row.geojson))
}

#[derive(Serialize)]
struct TrailWarnings {
    id: Uuid,
    name: Option<String>,
    osm_relation_id: i64,
    warnings: serde_json::Value,
}

#[get("/qa/warnings")]
async fn list_stitch_warnings(pool: web::Data<PgPool>) -> actix_web::Result<HttpResponse> {
    let rows = sqlx::query_as!(
        TrailWarnings,
        r#"
        SELECT t.id, t.name, t.osm_relation_id,
               jsonb_agg(w.message ORDER BY w.created_at) as "warnings!"
        FROM trails t
        JOIN stitch_warnings w ON w.trail_id = t.id
        GROUP BY t.id, t.name, t.osm_relation_id
        ORDER BY t.name
        "#
    )
    .fetch_all(pool.get_ref())
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(rows))
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
            .service(list_waypoints)
            .service(list_stitch_warnings)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}