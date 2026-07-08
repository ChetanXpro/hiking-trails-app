// Reads the staging tables (raw_ways, raw_relations) written by
// osm2pgsql's flex output, stitches each relation into a correctly
// ordered trail using stitch.rs, and writes the result into the real
// `trails` / `segments` tables from migration 0001.
//
// All queries use `sqlx::query_as!` / `sqlx::query!` (compile-time
// checked against a real, migrated database) rather than the
// runtime-only `sqlx::query()` + manual `.try_get("column")` pattern --
// see the comment at the top of hiking-api/src/main.rs for why that
// distinction matters, and what it requires from your build environment.
//
// Run with:
//   cargo run -p hiking-import   (reads DATABASE_URL from .env automatically)

mod stitch;

use anyhow::{Context, Result};
use hiking_db::decode_wkb;
use stitch::{stitch_ways, WayRecord};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
    let pool = hiking_db::create_pool(&database_url).await?;

    let relations = sqlx::query_as!(
        RawRelationRow,
        r#"
        SELECT osm_id,
               tags,
               member_ids as "member_ids: sqlx::types::Json<Vec<i64>>"
        FROM raw_relations
        "#
    )
    .fetch_all(&pool)
    .await
    .context("fetching raw_relations")?;

    tracing::info!(count = relations.len(), "stitching relations");

    let mut succeeded = 0usize;
    let mut failed = 0usize;

    for rel in relations {
        let osm_relation_id = rel.osm_id;
        let member_ids = rel.member_ids.0;

        match stitch_one_relation(&pool, osm_relation_id, &rel.tags, &member_ids).await {
            Ok(warnings) => {
                succeeded += 1;
                for w in warnings {
                    tracing::warn!(relation = osm_relation_id, "{w}");
                }
            }
            Err(e) => {
                failed += 1;
                tracing::error!(relation = osm_relation_id, error = %e, "failed to stitch relation");
            }
        }
    }

    tracing::info!(succeeded, failed, "stitching pass complete");

    let waypoint_count = import_waypoints(&pool).await?;
    tracing::info!(waypoint_count, "waypoint import complete");

    Ok(())
}

struct RawWaypointRow {
    osm_id: i64,
    tags: serde_json::Value,
    geom_wkb: Vec<u8>,
}

fn classify_waypoint(tags: &serde_json::Value) -> Option<&'static str> {
    let get = |key: &str| tags.get(key).and_then(|v| v.as_str());
    match (get("tourism"), get("natural")) {
        (Some("alpine_hut"), _) => Some("alpine_hut"),
        (Some("wilderness_hut"), _) => Some("wilderness_hut"),
        (Some("camp_site"), _) => Some("camp_site"),
        (_, Some("peak")) => Some("peak"),
        (_, Some("spring")) => Some("spring"),
        _ => None,
    }
}

async fn import_waypoints(pool: &sqlx::PgPool) -> Result<usize> {
    let rows = sqlx::query_as!(
        RawWaypointRow,
        r#"
        SELECT osm_id, tags, ST_AsBinary(geom) as "geom_wkb!"
        FROM raw_waypoints
        "#
    )
    .fetch_all(pool)
    .await
    .context("fetching raw_waypoints")?;

    sqlx::query!("DELETE FROM waypoints")
        .execute(pool)
        .await
        .context("clearing old waypoints")?;

    let mut inserted = 0usize;
    for row in &rows {
        let Some(waypoint_type) = classify_waypoint(&row.tags) else {
            continue;
        };
        let name = row.tags.get("name").and_then(|v| v.as_str());

        sqlx::query!(
            r#"
            INSERT INTO waypoints (id, osm_node_id, type, name, geom)
            VALUES ($1, $2, $3, $4, ST_SetSRID(ST_GeomFromWKB($5), 4326))
            "#,
            Uuid::new_v4(),
            row.osm_id,
            waypoint_type,
            name,
            row.geom_wkb
        )
        .execute(pool)
        .await
        .context("inserting waypoint")?;

        inserted += 1;
    }

    Ok(inserted)
}

struct RawRelationRow {
    osm_id: i64,
    tags: serde_json::Value,
    member_ids: sqlx::types::Json<Vec<i64>>,
}

struct RawWayRow {
    osm_id: i64,
    tags: serde_json::Value,
    geom_wkb: Vec<u8>,
    length_m: f64,
}

async fn stitch_one_relation(
    pool: &sqlx::PgPool,
    osm_relation_id: i64,
    tags: &serde_json::Value,
    member_ids: &[i64],
) -> Result<Vec<String>> {
    if member_ids.is_empty() {
        return Ok(vec!["relation has no member ways -- skipped".into()]);
    }

    let rows = sqlx::query_as!(
        RawWayRow,
        r#"
        SELECT osm_id,
               tags,
               ST_AsBinary(geom) as "geom_wkb!",
               ST_Length(geom::geography) as "length_m!"
        FROM raw_ways
        WHERE osm_id = ANY($1)
        "#,
        member_ids
    )
    .fetch_all(pool)
    .await
    .context("fetching member raw_ways")?;

    let mut ways = Vec::new();
    for row in &rows {
        let geom = decode_wkb(&row.geom_wkb)?;
        let coords: Vec<(f64, f64)> = match geom {
            geo_types::Geometry::LineString(ls) => {
                ls.points().map(|p| (p.x(), p.y())).collect()
            }
            other => anyhow::bail!("expected LineString for way {}, got {other:?}", row.osm_id),
        };
        let start = *coords.first().context("way has empty geometry")?;
        let end = *coords.last().context("way has empty geometry")?;

        ways.push(WayRecord {
            osm_way_id: row.osm_id,
            name: row.tags.get("name").and_then(|v| v.as_str()).map(String::from),
            surface: row.tags.get("surface").and_then(|v| v.as_str()).map(String::from),
            highway_type: row.tags.get("highway").and_then(|v| v.as_str()).map(String::from),
            length_m: row.length_m,
            start,
            end,
            geom_wkb: row.geom_wkb.clone(),
        });
    }

    let result = stitch_ways(&ways);

    let total_distance: f64 = result
        .ordered_ways
        .iter()
        .filter_map(|(id, _)| ways.iter().find(|w| w.osm_way_id == *id))
        .map(|w| w.length_m)
        .sum();

    let trail_name = tags.get("name").and_then(|v| v.as_str());
    let network = tags.get("network").and_then(|v| v.as_str());
    let trail_id = Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO trails (id, osm_relation_id, name, network, distance_m, last_synced_at)
        VALUES ($1, $2, $3, $4, $5, now())
        ON CONFLICT (osm_relation_id) DO UPDATE
        SET name = EXCLUDED.name,
            network = EXCLUDED.network,
            distance_m = EXCLUDED.distance_m,
            last_synced_at = now()
        "#,
        trail_id,
        osm_relation_id,
        trail_name,
        network,
        total_distance
    )
    .execute(pool)
    .await
    .context("upserting trail")?;

    sqlx::query!(
        "DELETE FROM segments WHERE trail_id = (SELECT id FROM trails WHERE osm_relation_id = $1)",
        osm_relation_id
    )
    .execute(pool)
    .await?;

    for (sequence, (way_id, reversed)) in result.ordered_ways.iter().enumerate() {
        let way = ways
            .iter()
            .find(|w| w.osm_way_id == *way_id)
            .context("stitched way not found in fetched set")?;

        sqlx::query!(
            r#"
            INSERT INTO segments (id, trail_id, osm_way_id, sequence, name, surface, highway_type, length_m, geom)
            VALUES (
                $1,
                (SELECT id FROM trails WHERE osm_relation_id = $2),
                $3, $4, $5, $6, $7, $8,
                CASE WHEN $10::boolean
                    THEN ST_Reverse(ST_SetSRID(ST_GeomFromWKB($9), 4326))
                    ELSE ST_SetSRID(ST_GeomFromWKB($9), 4326)
                END
            )
            "#,
            Uuid::new_v4(),
            osm_relation_id,
            way.osm_way_id,
            sequence as i32,
            way.name,
            way.surface,
            way.highway_type,
            way.length_m,
            way.geom_wkb,
            *reversed
        )
        .execute(pool)
        .await
        .context("inserting segment")?;
    }

    sqlx::query!(
        r#"
        UPDATE trails
        SET geom = (
            SELECT ST_Multi(ST_Collect(geom ORDER BY sequence))
            FROM segments
            WHERE trail_id = trails.id
        )
        WHERE osm_relation_id = $1
        "#,
        osm_relation_id
    )
    .execute(pool)
    .await
    .context("updating trail merged geometry")?;

    sqlx::query!(
        "DELETE FROM stitch_warnings WHERE trail_id = (SELECT id FROM trails WHERE osm_relation_id = $1)",
        osm_relation_id
    )
    .execute(pool)
    .await
    .context("clearing old stitch warnings")?;

    for warning in &result.warnings {
        sqlx::query!(
            r#"
            INSERT INTO stitch_warnings (id, trail_id, message)
            VALUES ($1, (SELECT id FROM trails WHERE osm_relation_id = $2), $3)
            "#,
            Uuid::new_v4(),
            osm_relation_id,
            warning
        )
        .execute(pool)
        .await
        .context("inserting stitch warning")?;
    }

    sqlx::query!(
        "DELETE FROM stitch_diagnostic_points WHERE trail_id = (SELECT id FROM trails WHERE osm_relation_id = $1)",
        osm_relation_id
    )
    .execute(pool)
    .await
    .context("clearing old diagnostic points")?;

    for point in &result.diagnostic_points {
        sqlx::query!(
            r#"
            INSERT INTO stitch_diagnostic_points (id, trail_id, kind, geom)
            VALUES ($1, (SELECT id FROM trails WHERE osm_relation_id = $2), $3, ST_SetSRID(ST_MakePoint($4, $5), 4326))
            "#,
            Uuid::new_v4(),
            osm_relation_id,
            point.kind.as_str(),
            point.coord.0,
            point.coord.1
        )
        .execute(pool)
        .await
        .context("inserting diagnostic point")?;
    }

    Ok(result.warnings)
}