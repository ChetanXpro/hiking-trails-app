// Model structs mirroring the schema in migrations/0001_init_schema.sql.
//
// Geometry columns (`boundary`, `geom`) are deliberately NOT part of these
// structs. sqlx has no built-in PostGIS geometry decoder, and the common,
// well-supported pattern is: fetch the non-spatial columns directly via
// `FromRow`, and fetch geometry separately by wrapping it in the query as
// `ST_AsBinary(col)` (raw WKB), then decoding with `decode_wkb` below.
// This also keeps "just show me the trail list" queries fast, since you
// only pay the geometry-decoding cost when you actually need the shape.

use chrono::{DateTime, Utc};
use geo_types::Geometry;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool, Row};
use uuid::Uuid;

/// Decode a PostGIS geometry column fetched as raw WKB bytes
/// (via `ST_AsBinary(col)` in your SQL) into a geo-types geometry.
pub fn decode_wkb(bytes: &[u8]) -> anyhow::Result<Geometry<f64>> {
    wkb::wkb_to_geom(&mut std::io::Cursor::new(bytes))
        .map_err(|e| anyhow::anyhow!("failed to decode WKB geometry: {e:?}"))
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Region {
    pub id: Uuid,
    pub osm_relation_id: Option<i64>,
    pub name: String,
    pub admin_level: Option<i16>,
    pub country_code: String,
    pub parent_region_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Trail {
    pub id: Uuid,
    pub osm_relation_id: i64,
    pub region_id: Option<Uuid>,
    pub name: Option<String>,
    pub network: Option<String>,
    pub difficulty: Option<String>,
    pub distance_m: Option<f64>,
    pub ascent_m: Option<f64>,
    pub descent_m: Option<f64>,
    pub summary: Option<String>,
    pub hike_type: Option<String>,
    pub estimated_duration_min: Option<i32>,
    pub default_direction_mode: String,
    pub direction_source: Option<String>,
    pub source_kind: String,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Segment {
    pub id: Uuid,
    pub trail_id: Uuid,
    pub osm_way_id: i64,
    pub sequence: i32,
    pub name: Option<String>,
    pub surface: Option<String>,
    pub highway_type: Option<String>,
    pub length_m: Option<f64>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Waypoint {
    pub id: Uuid,
    pub trail_id: Option<Uuid>,
    pub osm_node_id: Option<i64>,
    #[sqlx(rename = "type")]
    pub kind: String,
    pub name: Option<String>,
    pub elevation_m: Option<f64>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TrailSurfaceBreakdown {
    pub trail_id: Uuid,
    pub surface_type: String,
    pub distance_m: f64,
    pub pct: f64,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TrailFeature {
    pub trail_id: Uuid,
    pub feature_slug: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElevationPoint {
    pub pos_m: f64,
    pub ele_m: f64,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ElevationProfile {
    pub id: Uuid,
    pub trail_id: Uuid,
    pub min_elevation_m: Option<f64>,
    pub max_elevation_m: Option<f64>,
    pub profile_points: sqlx::types::Json<Vec<ElevationPoint>>,
    pub computed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SyncState {
    pub id: Uuid,
    pub source_name: String,
    pub last_sequence_number: Option<i64>,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub replication_url: Option<String>,
}

/// Example: fetch a trail's non-spatial fields plus its merged route
/// geometry in one query, decoding the WKB into a geo-types Geometry.
/// This is the pattern to follow for any query that needs the shape --
/// list/search endpoints should skip the geometry select entirely.
pub async fn fetch_trail_with_geometry(
    pool: &PgPool,
    trail_id: Uuid,
) -> anyhow::Result<(Trail, Option<Geometry<f64>>)> {
    let row = sqlx::query(
        r#"
        SELECT id, osm_relation_id, region_id, name, network, difficulty,
               distance_m, ascent_m, descent_m, summary, hike_type,
               estimated_duration_min, default_direction_mode, direction_source,
               source_kind, last_synced_at,
               created_at, updated_at,
               ST_AsBinary(geom) AS geom_wkb
        FROM trails
        WHERE id = $1
        "#,
    )
    .bind(trail_id)
    .fetch_one(pool)
    .await?;

    let trail = Trail {
        id: row.try_get("id")?,
        osm_relation_id: row.try_get("osm_relation_id")?,
        region_id: row.try_get("region_id")?,
        name: row.try_get("name")?,
        network: row.try_get("network")?,
        difficulty: row.try_get("difficulty")?,
        distance_m: row.try_get("distance_m")?,
        ascent_m: row.try_get("ascent_m")?,
        descent_m: row.try_get("descent_m")?,
        summary: row.try_get("summary")?,
        hike_type: row.try_get("hike_type")?,
        estimated_duration_min: row.try_get("estimated_duration_min")?,
        default_direction_mode: row.try_get("default_direction_mode")?,
        direction_source: row.try_get("direction_source")?,
        source_kind: row.try_get("source_kind")?,
        last_synced_at: row.try_get("last_synced_at")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    };

    let geom_bytes: Option<Vec<u8>> = row.try_get("geom_wkb")?;
    let geom = geom_bytes.map(|b| decode_wkb(&b)).transpose()?;

    Ok((trail, geom))
}
