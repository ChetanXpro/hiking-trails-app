mod stitch;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use hiking_db::decode_wkb;
use serde::Deserialize;
use serde_json::Value;
use sqlx::{types::Json, FromRow, Postgres, QueryBuilder};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use stitch::{stitch_ways, WayRecord};
use uuid::Uuid;

const DEFAULT_ELEVATION_SAMPLE_SPACING_M: f64 = 30.0;
const MIN_ELEVATION_SAMPLES: usize = 24;
const DEFAULT_ELEVATION_SMOOTHING_WINDOW: usize = 5;
const DEFAULT_GAIN_LOSS_THRESHOLD_M: f64 = 10.0;
const MAX_ORIGINAL_MAPPER_MANIFEST_BYTES: u64 = 5 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OriginalMapperManifest {
    schema_version: u32,
    relations: Vec<OriginalMapperManifestEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OriginalMapperManifestEntry {
    osm_relation_id: i64,
    osm_relation_version: u32,
    display_name: String,
    provider_user_id: Option<String>,
    profile_url: Option<String>,
    mapped_at: DateTime<Utc>,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
    let pool = hiking_db::create_pool(&database_url).await?;
    ensure_expected_schema(&pool).await?;
    sync_regions_and_places(&pool).await?;
    sync_outdoor_waypoints(&pool).await?;
    let elevation_sampler = ElevationSampler::from_env()?;
    let landcover_sampler = LandcoverSampler::from_env()?;

    if let Some(sampler) = &elevation_sampler {
        tracing::info!(
            dem_tiles = sampler.dem_sources.len(),
            "elevation sampling enabled"
        );
    } else {
        tracing::info!("DEM_PATH / DEM_PATHS not set -- skipping elevation profile generation");
    }
    if let Some(sampler) = &landcover_sampler {
        tracing::info!(
            worldcover_path = %sampler.worldcover_source.path.display(),
            "WorldCover terrain tagging enabled"
        );
    } else {
        tracing::info!("WORLDCOVER_PATH not set -- skipping terrain segment tagging");
    }

    let relations = sqlx::query_as::<_, RawRelationRow>(
        r#"
        SELECT
            osm_id,
            tags,
            members
        FROM raw_relations
        ORDER BY osm_id
        "#,
    )
    .fetch_all(&pool)
    .await
    .context("fetching raw_relations")?;
    let relation_lookup = relations
        .iter()
        .cloned()
        .map(|relation| (relation.osm_id, relation))
        .collect::<HashMap<_, _>>();

    tracing::info!(count = relations.len(), "stitching relations");

    let mut succeeded = 0usize;
    let mut failed = 0usize;

    for rel in relations {
        let osm_relation_id = rel.osm_id;
        let members = rel.members.0;

        match stitch_one_relation(
            &pool,
            elevation_sampler.as_ref(),
            landcover_sampler.as_ref(),
            osm_relation_id,
            &rel.tags,
            &members,
            &relation_lookup,
        )
        .await
        {
            Ok(warnings) => {
                succeeded += 1;
                for warning in warnings {
                    tracing::warn!(relation = osm_relation_id, "{warning}");
                }
            }
            Err(error) => {
                failed += 1;
                tracing::error!(
                    relation = osm_relation_id,
                    error = %format!("{error:#}"),
                    "failed to stitch relation"
                );
            }
        }
    }

    tracing::info!(succeeded, failed, "stitching pass complete");
    sync_original_mapper_manifest_from_env(&pool).await?;
    Ok(())
}

async fn ensure_expected_schema(pool: &sqlx::PgPool) -> Result<()> {
    let landcover_table = sqlx::query_scalar::<_, Option<String>>(
        "SELECT to_regclass('public.trail_landcover_segments')::text",
    )
    .fetch_one(pool)
    .await
    .context("checking expected schema objects")?;

    if landcover_table.is_none() {
        anyhow::bail!(
            "database schema is missing 'trail_landcover_segments' (migration 0007). Run `sqlx migrate run` and then rerun `cargo run -p hiking-import`"
        );
    }

    let places_table =
        sqlx::query_scalar::<_, Option<String>>("SELECT to_regclass('public.places')::text")
            .fetch_one(pool)
            .await
            .context("checking places schema object")?;

    if places_table.is_none() {
        anyhow::bail!(
            "database schema is missing 'places' (migration 0010). Run `sqlx migrate run` and then rerun `cargo run -p hiking-import`"
        );
    }

    let has_original_mapper_columns = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT COUNT(*) = 6
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'trails'
          AND column_name IN (
              'original_mapper_name',
              'original_mapper_provider',
              'original_mapper_user_id',
              'original_mapper_profile_url',
              'original_mapped_at',
              'original_mapper_source'
          )
        "#,
    )
    .fetch_one(pool)
    .await
    .context("checking original mapper attribution schema")?;

    if !has_original_mapper_columns {
        anyhow::bail!(
            "database schema is missing original mapper attribution columns (migration 0013). Run `sqlx migrate run` and then rerun `cargo run -p hiking-import`"
        );
    }

    let outdoor_waypoints_table = sqlx::query_scalar::<_, Option<String>>(
        "SELECT to_regclass('public.outdoor_waypoints')::text",
    )
    .fetch_one(pool)
    .await
    .context("checking outdoor waypoint schema object")?;

    if outdoor_waypoints_table.is_none() {
        anyhow::bail!(
            "database schema is missing 'outdoor_waypoints' (migration 0012). Run `sqlx migrate run` and then rerun `cargo run -p hiking-import`"
        );
    }

    ensure_raw_osm_identity_column(pool, "raw_waypoints").await?;
    ensure_raw_osm_identity_column(pool, "raw_places").await?;

    Ok(())
}

fn parse_original_mapper_manifest(bytes: &[u8]) -> Result<OriginalMapperManifest> {
    let mut manifest: OriginalMapperManifest =
        serde_json::from_slice(bytes).context("parsing original mapper manifest JSON")?;

    if manifest.schema_version != 1 {
        anyhow::bail!(
            "unsupported original mapper manifest schema_version {}; expected 1",
            manifest.schema_version
        );
    }

    let mut relation_ids = HashSet::with_capacity(manifest.relations.len());
    for entry in &mut manifest.relations {
        if entry.osm_relation_id <= 0 {
            anyhow::bail!("original mapper manifest contains an invalid OSM relation id");
        }
        if entry.osm_relation_version != 1 {
            anyhow::bail!(
                "original mapper entry for relation {} is version {}; only relation version 1 proves original map authorship",
                entry.osm_relation_id,
                entry.osm_relation_version
            );
        }
        if !relation_ids.insert(entry.osm_relation_id) {
            anyhow::bail!(
                "original mapper manifest contains relation {} more than once",
                entry.osm_relation_id
            );
        }

        entry.display_name = entry.display_name.trim().to_string();
        if entry.display_name.is_empty() || entry.display_name.len() > 255 {
            anyhow::bail!(
                "original mapper entry for relation {} has an invalid display_name",
                entry.osm_relation_id
            );
        }

        entry.provider_user_id = entry
            .provider_user_id
            .take()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        if let Some(profile_url) = entry.profile_url.as_mut() {
            *profile_url = profile_url.trim().to_string();
            if !profile_url.starts_with("https://www.openstreetmap.org/user/") {
                anyhow::bail!(
                    "original mapper entry for relation {} has a profile_url outside openstreetmap.org",
                    entry.osm_relation_id
                );
            }
        }
    }

    Ok(manifest)
}

async fn sync_original_mapper_manifest_from_env(pool: &sqlx::PgPool) -> Result<()> {
    let Some(path) = std::env::var("OSM_ORIGINAL_MAPPERS_PATH")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    else {
        tracing::info!(
            "OSM_ORIGINAL_MAPPERS_PATH not set -- original OSM relation authorship remains unchanged"
        );
        return Ok(());
    };

    let metadata = fs::metadata(&path)
        .with_context(|| format!("reading original mapper manifest metadata from {path}"))?;
    if metadata.len() > MAX_ORIGINAL_MAPPER_MANIFEST_BYTES {
        anyhow::bail!(
            "original mapper manifest is larger than {} bytes",
            MAX_ORIGINAL_MAPPER_MANIFEST_BYTES
        );
    }

    let bytes =
        fs::read(&path).with_context(|| format!("reading original mapper manifest from {path}"))?;
    let manifest = parse_original_mapper_manifest(&bytes)?;
    let listed = manifest.relations.len();

    let mut tx = pool
        .begin()
        .await
        .context("starting original mapper manifest transaction")?;
    let mut matched = 0usize;

    for entry in manifest.relations {
        let result = sqlx::query(
            r#"
            UPDATE trails
            SET original_mapper_name = $2,
                original_mapper_provider = 'osm',
                original_mapper_user_id = $3,
                original_mapper_profile_url = $4,
                original_mapped_at = $5,
                original_mapper_source = 'osm_relation_version_1_manifest',
                updated_at = now()
            WHERE osm_relation_id = $1
            "#,
        )
        .bind(entry.osm_relation_id)
        .bind(entry.display_name)
        .bind(entry.provider_user_id)
        .bind(entry.profile_url)
        .bind(entry.mapped_at)
        .execute(&mut *tx)
        .await
        .with_context(|| {
            format!(
                "applying original mapper attribution for relation {}",
                entry.osm_relation_id
            )
        })?;

        matched += result.rows_affected() as usize;
    }

    tx.commit()
        .await
        .context("committing original mapper manifest")?;
    tracing::info!(
        path,
        listed,
        matched,
        unmatched = listed.saturating_sub(matched),
        "original OSM relation authorship manifest applied"
    );

    Ok(())
}

async fn ensure_raw_osm_identity_column(pool: &sqlx::PgPool, table_name: &str) -> Result<()> {
    let has_osm_type = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM information_schema.columns
            WHERE table_schema = 'public'
              AND table_name = $1
              AND column_name = 'osm_type'
        )
        "#,
    )
    .bind(table_name)
    .fetch_one(pool)
    .await
    .with_context(|| format!("checking {table_name} OSM identity column"))?;

    if raw_table_exists(pool, table_name).await? && !has_osm_type {
        anyhow::bail!(
            "{table_name} uses the old node-only staging schema. Re-run osm2pgsql with the updated `osm2pgsql/trails.lua`, then rerun `cargo run -p hiking-import`."
        );
    }

    Ok(())
}

async fn sync_regions_and_places(pool: &sqlx::PgPool) -> Result<()> {
    sync_regions(pool).await?;
    sync_places(pool).await?;
    Ok(())
}

async fn sync_regions(pool: &sqlx::PgPool) -> Result<()> {
    if !raw_table_exists(pool, "raw_regions").await? {
        tracing::info!("raw_regions staging table not present -- skipping region hierarchy sync");
        return Ok(());
    }

    let raw_regions = sqlx::query_as::<_, RawRegionRow>(
        r#"
        SELECT
            osm_id,
            tags,
            ST_AsBinary(
                ST_Multi(ST_CollectionExtract(ST_MakeValid(geom), 3))
            ) AS geom_wkb
        FROM raw_regions
        ORDER BY osm_id
        "#,
    )
    .fetch_all(pool)
    .await
    .context("fetching raw_regions")?;

    if raw_regions.is_empty() {
        tracing::info!("raw_regions staging table is empty -- skipping region hierarchy sync");
        return Ok(());
    }

    let region_rows = raw_regions
        .into_iter()
        .filter_map(|region| {
            let name = tag_str(&region.tags, "name")?.to_owned();
            Some((
                region.osm_id,
                name,
                parse_admin_level_tag(&region.tags),
                tag_str(&region.tags, "boundary")
                    .unwrap_or("administrative")
                    .to_owned(),
                extract_country_code(&region.tags),
                region.geom_wkb,
            ))
        })
        .collect::<Vec<_>>();

    if region_rows.is_empty() {
        tracing::info!("no named administrative regions found in raw_regions");
        return Ok(());
    }

    let mut tx = pool
        .begin()
        .await
        .context("starting region sync transaction")?;

    sqlx::query("UPDATE regions SET parent_region_id = NULL")
        .execute(&mut *tx)
        .await
        .context("clearing region parent pointers before sync")?;

    for chunk in region_rows.chunks(500) {
        let mut query = QueryBuilder::<Postgres>::new(
            r#"
            INSERT INTO regions (
                osm_relation_id,
                name,
                admin_level,
                boundary_kind,
                country_code,
                boundary
            )
            "#,
        );

        query.push_values(chunk, |mut builder, row| {
            builder
                .push_bind(row.0)
                .push_bind(&row.1)
                .push_bind(row.2)
                .push_bind(&row.3)
                .push_bind(&row.4);

            builder
                .push_unseparated(
                    ", ST_Multi(ST_CollectionExtract(ST_MakeValid(ST_SetSRID(ST_GeomFromWKB(",
                )
                .push_bind_unseparated(&row.5)
                .push_unseparated("), 4326)), 3))");
        });

        query.push(
            r#"
            ON CONFLICT (osm_relation_id) DO UPDATE
            SET name = EXCLUDED.name,
                admin_level = EXCLUDED.admin_level,
                boundary_kind = EXCLUDED.boundary_kind,
                country_code = COALESCE(EXCLUDED.country_code, regions.country_code),
                boundary = EXCLUDED.boundary,
                updated_at = now()
            "#,
        );

        query
            .build()
            .execute(&mut *tx)
            .await
            .context("upserting administrative regions")?;
    }

    sqlx::query(
        r#"
        UPDATE regions child
        SET parent_region_id = (
                SELECT parent.id
                FROM regions parent
                WHERE parent.id <> child.id
                  AND parent.boundary IS NOT NULL
                  AND child.boundary IS NOT NULL
                  AND parent.admin_level IS NOT NULL
                  AND (
                      child.admin_level IS NULL
                      OR parent.admin_level < child.admin_level
                  )
                  AND ST_Covers(parent.boundary, ST_PointOnSurface(child.boundary))
                ORDER BY parent.admin_level DESC, ST_Area(parent.boundary) ASC
                LIMIT 1
            ),
            country_code = COALESCE(
                child.country_code,
                (
                    SELECT parent.country_code
                    FROM regions parent
                    WHERE parent.id <> child.id
                      AND parent.boundary IS NOT NULL
                      AND child.boundary IS NOT NULL
                      AND parent.admin_level IS NOT NULL
                      AND (
                          child.admin_level IS NULL
                          OR parent.admin_level < child.admin_level
                      )
                      AND ST_Covers(parent.boundary, ST_PointOnSurface(child.boundary))
                    ORDER BY parent.admin_level DESC, ST_Area(parent.boundary) ASC
                    LIMIT 1
                )
            ),
            updated_at = now()
        WHERE child.boundary IS NOT NULL
        "#,
    )
    .execute(&mut *tx)
    .await
    .context("linking region parent hierarchy")?;

    tx.commit()
        .await
        .context("committing region hierarchy sync")?;
    tracing::info!(count = region_rows.len(), "region hierarchy synced");
    Ok(())
}

async fn sync_places(pool: &sqlx::PgPool) -> Result<()> {
    if !raw_table_exists(pool, "raw_places").await? {
        tracing::info!("raw_places staging table not present -- skipping named-place sync");
        return Ok(());
    }

    let raw_places = sqlx::query_as::<_, RawPlaceRow>(
        r#"
        SELECT osm_type::text AS osm_type, osm_id, tags, ST_AsBinary(geom) AS geom_wkb
        FROM raw_places
        ORDER BY osm_id
        "#,
    )
    .fetch_all(pool)
    .await
    .context("fetching raw_places")?;

    if raw_places.is_empty() {
        tracing::info!("raw_places staging table is empty -- skipping named-place sync");
        return Ok(());
    }

    let place_rows = raw_places
        .into_iter()
        .filter_map(|place| {
            let name = tag_str(&place.tags, "name")?.to_owned();
            let place_kind = tag_str(&place.tags, "place")?.to_owned();
            Some((
                osm_type_name(&place.osm_type)?.to_string(),
                place.osm_id,
                name,
                place_kind,
                extract_country_code(&place.tags),
                place.geom_wkb,
            ))
        })
        .collect::<Vec<_>>();

    if place_rows.is_empty() {
        tracing::info!("no named places found in raw_places");
        return Ok(());
    }

    let mut tx = pool
        .begin()
        .await
        .context("starting named-place sync transaction")?;

    for chunk in place_rows.chunks(1000) {
        let mut query = QueryBuilder::<Postgres>::new(
            r#"
            INSERT INTO places (
                osm_type,
                osm_id,
                name,
                place_kind,
                country_code,
                geom
            )
            "#,
        );

        query.push_values(chunk, |mut builder, row| {
            builder
                .push_bind(&row.0)
                .push_bind(row.1)
                .push_bind(&row.2)
                .push_bind(&row.3)
                .push_bind(&row.4);

            builder
                .push_unseparated(", ST_SetSRID(ST_GeomFromWKB(")
                .push_bind_unseparated(&row.5)
                .push_unseparated("), 4326)");
        });

        query.push(
            r#"
            ON CONFLICT (osm_type, osm_id) DO UPDATE
            SET name = EXCLUDED.name,
                place_kind = EXCLUDED.place_kind,
                country_code = COALESCE(EXCLUDED.country_code, places.country_code),
                geom = EXCLUDED.geom,
                updated_at = now()
            "#,
        );

        query
            .build()
            .execute(&mut *tx)
            .await
            .context("upserting named places")?;
    }

    sqlx::query(
        r#"
        UPDATE places
        SET region_id = NULL
        "#,
    )
    .execute(&mut *tx)
    .await
    .context("clearing place region assignments before sync")?;

    sqlx::query(
        r#"
        UPDATE places place
        SET region_id = (
                SELECT region.id
                FROM regions region
                WHERE region.boundary IS NOT NULL
                  AND ST_Covers(region.boundary, place.geom)
                ORDER BY region.admin_level DESC NULLS LAST, ST_Area(region.boundary) ASC
                LIMIT 1
            ),
            country_code = COALESCE(
                place.country_code,
                (
                    SELECT region.country_code
                    FROM regions region
                    WHERE region.boundary IS NOT NULL
                      AND ST_Covers(region.boundary, place.geom)
                    ORDER BY region.admin_level DESC NULLS LAST, ST_Area(region.boundary) ASC
                    LIMIT 1
                )
            ),
            updated_at = now()
        "#,
    )
    .execute(&mut *tx)
    .await
    .context("assigning places to containing regions")?;

    tx.commit().await.context("committing named-place sync")?;
    tracing::info!(count = place_rows.len(), "named places synced");
    Ok(())
}

async fn sync_outdoor_waypoints(pool: &sqlx::PgPool) -> Result<()> {
    if !raw_table_exists(pool, "raw_waypoints").await? {
        tracing::info!("raw_waypoints staging table not present -- skipping outdoor waypoint sync");
        return Ok(());
    }

    let raw_waypoints = sqlx::query_as::<_, RawWaypointRow>(
        r#"
        SELECT osm_type::text AS osm_type, osm_id, tags, ST_AsBinary(geom) AS geom_wkb
        FROM raw_waypoints
        ORDER BY osm_type, osm_id
        "#,
    )
    .fetch_all(pool)
    .await
    .context("fetching outdoor waypoints")?;

    let rows = raw_waypoints
        .into_iter()
        .filter_map(|waypoint| {
            let osm_type = normalized_osm_type(&waypoint.osm_type)?.to_string();
            let kind = infer_waypoint_kind(&waypoint.tags).to_string();
            let name = tag_str(&waypoint.tags, "name").map(str::to_owned);
            let elevation_m = tag_str(&waypoint.tags, "ele").and_then(parse_elevation);
            Some((
                osm_type,
                waypoint.osm_id,
                kind,
                name,
                elevation_m,
                waypoint.geom_wkb,
            ))
        })
        .collect::<Vec<_>>();

    let mut tx = pool
        .begin()
        .await
        .context("starting outdoor waypoint sync transaction")?;

    sqlx::query("DELETE FROM outdoor_waypoints")
        .execute(&mut *tx)
        .await
        .context("clearing outdoor waypoints before sync")?;

    for chunk in rows.chunks(1000) {
        let mut query = QueryBuilder::<Postgres>::new(
            r#"
            INSERT INTO outdoor_waypoints (
                osm_type,
                osm_id,
                kind,
                name,
                elevation_m,
                geom
            )
            "#,
        );

        query.push_values(chunk, |mut builder, row| {
            builder
                .push_bind(&row.0)
                .push_bind(row.1)
                .push_bind(&row.2)
                .push_bind(&row.3)
                .push_bind(row.4);

            builder
                .push_unseparated(", ST_SetSRID(ST_GeomFromWKB(")
                .push_bind_unseparated(&row.5)
                .push_unseparated("), 4326)");
        });

        query
            .build()
            .execute(&mut *tx)
            .await
            .context("inserting outdoor waypoints")?;
    }

    tx.commit()
        .await
        .context("committing outdoor waypoint sync")?;
    tracing::info!(count = rows.len(), "outdoor waypoints synced");
    Ok(())
}

async fn raw_table_exists(pool: &sqlx::PgPool, table_name: &str) -> Result<bool> {
    let table = sqlx::query_scalar::<_, Option<String>>("SELECT to_regclass($1)::text")
        .bind(format!("public.{table_name}"))
        .fetch_one(pool)
        .await
        .with_context(|| format!("checking staging table existence for {table_name}"))?;

    Ok(table.is_some())
}

#[derive(Debug, Clone, FromRow)]
struct RawRelationRow {
    osm_id: i64,
    tags: Value,
    members: Json<Vec<RawRelationMember>>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct RawRelationMember {
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "ref")]
    ref_id: i64,
    #[serde(default)]
    role: String,
}

#[derive(Debug, Clone, FromRow)]
struct RawWayRow {
    osm_id: i64,
    tags: Value,
    geom_wkb: Vec<u8>,
    length_m: f64,
}

#[derive(Debug, Clone, FromRow)]
struct RawWaypointRow {
    osm_type: String,
    osm_id: i64,
    tags: Value,
    geom_wkb: Vec<u8>,
}

#[derive(Debug, Clone, FromRow)]
struct RawRegionRow {
    osm_id: i64,
    tags: Value,
    geom_wkb: Vec<u8>,
}

#[derive(Debug, Clone, FromRow)]
struct RawPlaceRow {
    osm_type: String,
    osm_id: i64,
    tags: Value,
    geom_wkb: Vec<u8>,
}

#[derive(Debug, Clone)]
struct PreparedWay {
    start: (f64, f64),
    end: (f64, f64),
}

#[derive(Debug, Clone)]
struct PreparedRelationMember {
    way_id: i64,
    role: String,
}

#[derive(Debug)]
struct RelationBuild {
    ways: Vec<WayRecord>,
    warnings: Vec<String>,
    missing_way_members: usize,
    had_direction_roles: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
struct ElevationPoint {
    pos_m: f64,
    ele_m: f64,
}

#[derive(Debug, Clone)]
struct ComputedElevationProfile {
    min_elevation_m: f64,
    max_elevation_m: f64,
    ascent_m: f64,
    descent_m: f64,
    profile_points: Vec<ElevationPoint>,
}

#[derive(Debug, Clone)]
struct RasterSource {
    path: PathBuf,
    nodata_value: Option<f64>,
    bounds_wgs84: Option<RasterBounds>,
    nominal_resolution_m: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
struct GdalCapabilities {
    supports_batch_queries: bool,
    supports_interpolation_flag: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct RasterBounds {
    min_lon: f64,
    min_lat: f64,
    max_lon: f64,
    max_lat: f64,
}

#[derive(Debug, Clone)]
struct ElevationSampler {
    dem_sources: Vec<RasterSource>,
    gdal_capabilities: GdalCapabilities,
    sample_spacing_m: f64,
    smoothing_window: usize,
    gain_loss_threshold_m: f64,
}

#[derive(Debug, Clone)]
struct LandcoverSampler {
    worldcover_source: RasterSource,
    gdal_capabilities: GdalCapabilities,
}

#[derive(Debug, Clone)]
struct LandcoverSegment {
    start_m: f64,
    end_m: f64,
    landcover_code: i32,
    landcover_label: &'static str,
    dem_confidence: &'static str,
}

async fn stitch_one_relation(
    pool: &sqlx::PgPool,
    elevation_sampler: Option<&ElevationSampler>,
    landcover_sampler: Option<&LandcoverSampler>,
    osm_relation_id: i64,
    tags: &Value,
    members: &[RawRelationMember],
    relation_lookup: &HashMap<i64, RawRelationRow>,
) -> Result<Vec<String>> {
    let mut staged_warnings = Vec::new();
    let member_ids = collect_relation_way_ids(
        osm_relation_id,
        members,
        relation_lookup,
        &mut vec![osm_relation_id],
        &mut staged_warnings,
    );

    if member_ids.is_empty() {
        return Ok(vec!["relation has no member ways -- skipped".into()]);
    }
    let unique_member_ids = member_ids
        .iter()
        .copied()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let rows = sqlx::query_as::<_, RawWayRow>(
        r#"
        SELECT osm_id,
               tags,
               ST_AsBinary(geom) AS geom_wkb,
               ST_Length(geom::geography) AS length_m
        FROM raw_ways
        WHERE osm_id = ANY($1)
        "#,
    )
    .bind(unique_member_ids)
    .fetch_all(pool)
    .await
    .context("fetching member raw_ways")?;

    if rows.is_empty() {
        return Ok(vec![
            "no member ways from this relation were found in raw_ways -- skipped".into(),
        ]);
    }

    let (prepared_by_id, way_lookup) = prepare_way_maps(rows)?;
    let relation_build = build_relation_way_records(
        osm_relation_id,
        members,
        relation_lookup,
        &prepared_by_id,
        &mut vec![osm_relation_id],
    )?;

    let ways = relation_build.ways;
    let result = stitch_ways(&ways, relation_build.had_direction_roles);
    if result.ordered_ways.is_empty() {
        return Ok(vec![
            "stitching produced no ordered ways for this relation -- skipped".into(),
        ]);
    }

    let ignored_non_way_members = members
        .iter()
        .filter(|member| {
            !member.kind.eq_ignore_ascii_case("w") && !member.kind.eq_ignore_ascii_case("r")
        })
        .count();

    let mut warnings = staged_warnings;
    warnings.extend(relation_build.warnings);
    warnings.extend(result.warnings.clone());
    if relation_build.missing_way_members > 0 {
        warnings.push(format!(
            "{} relation member way(s) were missing from raw_ways -- the extract/import filter dropped part of this route, so geometry and distance may be incomplete",
            relation_build.missing_way_members
        ));
    }
    if ignored_non_way_members > 0 {
        warnings.push(format!(
            "{ignored_non_way_members} non-way relation member(s) were ignored while stitching -- they were preserved in raw staging but are not yet part of the route builder"
        ));
    }
    warnings = dedupe_preserve_order(warnings);

    let trail_name = tag_str(tags, "name");
    let network = tag_str(tags, "network");
    let summary = tag_str(tags, "description").or_else(|| tag_str(tags, "note"));
    let difficulty = derive_difficulty(tags, &way_lookup);
    let total_distance: f64 = result
        .ordered_ways
        .iter()
        .filter_map(|(id, _)| way_lookup.get(id))
        .map(|way| way.length_m)
        .sum();
    let estimated_duration_min =
        estimate_duration_minutes(total_distance, 0.0, 0.0, difficulty.as_deref())
            .map(|value| value as i32);
    let default_direction_mode = result.default_direction_mode.as_str();
    let route_coordinates = ordered_route_coordinates(&result.ordered_ways, &way_lookup)
        .context("building ordered route coordinates")?;
    let elevation_profile = if let Some(sampler) = elevation_sampler {
        match sampler.sample_route(&route_coordinates) {
            Ok(profile) => Some(profile),
            Err(error) => {
                warnings.push(format!(
                    "failed to generate elevation profile from DEM -- {}",
                    error
                ));
                None
            }
        }
    } else {
        None
    };
    let landcover_segments = if let Some(sampler) = landcover_sampler {
        match sampler.segment_route(&route_coordinates) {
            Ok(segments) => segments,
            Err(error) => {
                warnings.push(format!(
                    "failed to generate WorldCover terrain segments -- {}",
                    error
                ));
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };
    let mut tx = pool.begin().await.context("starting transaction")?;

    let trail_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO trails (
            id,
            osm_relation_id,
            name,
            network,
            difficulty,
            distance_m,
            summary,
            hike_type,
            estimated_duration_min,
            default_direction_mode,
            direction_source,
            source_kind,
            last_synced_at,
            updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 'osm', now(), now())
        ON CONFLICT (osm_relation_id) DO UPDATE
        SET name = EXCLUDED.name,
            network = EXCLUDED.network,
            difficulty = EXCLUDED.difficulty,
            distance_m = EXCLUDED.distance_m,
            summary = EXCLUDED.summary,
            hike_type = EXCLUDED.hike_type,
            estimated_duration_min = EXCLUDED.estimated_duration_min,
            default_direction_mode = EXCLUDED.default_direction_mode,
            direction_source = EXCLUDED.direction_source,
            last_synced_at = now(),
            updated_at = now()
        RETURNING id
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(osm_relation_id)
    .bind(trail_name)
    .bind(network)
    .bind(difficulty.as_deref())
    .bind(total_distance)
    .bind(summary)
    .bind(Some(result.hike_type.as_str()))
    .bind(estimated_duration_min)
    .bind(default_direction_mode)
    .bind(Some(result.direction_source.as_str()))
    .fetch_one(&mut *tx)
    .await
    .context("upserting trail")?;

    sqlx::query("DELETE FROM segments WHERE trail_id = $1")
        .bind(trail_id)
        .execute(&mut *tx)
        .await
        .context("deleting prior segments")?;

    sqlx::query("DELETE FROM waypoints WHERE trail_id = $1")
        .bind(trail_id)
        .execute(&mut *tx)
        .await
        .context("deleting prior waypoints")?;

    sqlx::query("DELETE FROM trail_surface_breakdown WHERE trail_id = $1")
        .bind(trail_id)
        .execute(&mut *tx)
        .await
        .context("deleting prior surface breakdown")?;

    sqlx::query("DELETE FROM trail_features WHERE trail_id = $1")
        .bind(trail_id)
        .execute(&mut *tx)
        .await
        .context("deleting prior trail features")?;

    sqlx::query("DELETE FROM trail_landcover_segments WHERE trail_id = $1")
        .bind(trail_id)
        .execute(&mut *tx)
        .await
        .context("deleting prior trail landcover segments")?;

    sqlx::query("DELETE FROM stitch_warnings WHERE trail_id = $1")
        .bind(trail_id)
        .execute(&mut *tx)
        .await
        .context("deleting prior stitch warnings")?;

    sqlx::query("DELETE FROM stitch_diagnostic_points WHERE trail_id = $1")
        .bind(trail_id)
        .execute(&mut *tx)
        .await
        .context("deleting prior stitch diagnostic points")?;

    sqlx::query("DELETE FROM elevation_profiles WHERE trail_id = $1")
        .bind(trail_id)
        .execute(&mut *tx)
        .await
        .context("deleting prior elevation profile")?;

    let mut surface_totals: HashMap<&'static str, f64> = HashMap::new();
    let mut segment_rows = Vec::with_capacity(result.ordered_ways.len());
    for (sequence, (way_id, reversed)) in result.ordered_ways.iter().enumerate() {
        let way = way_lookup
            .get(way_id)
            .context("stitched way not found in fetched set")?;

        segment_rows.push((
            Uuid::new_v4(),
            trail_id,
            way.osm_id,
            sequence as i32,
            tag_str(&way.tags, "name").map(str::to_owned),
            tag_str(&way.tags, "surface").map(str::to_owned),
            tag_str(&way.tags, "highway").map(str::to_owned),
            way.length_m,
            *reversed,
            way.geom_wkb.clone(),
        ));

        let bucket =
            normalize_surface(tag_str(&way.tags, "surface"), tag_str(&way.tags, "highway"));
        *surface_totals.entry(bucket).or_insert(0.0) += way.length_m;
    }
    insert_segments_batch(&mut tx, &segment_rows).await?;

    sqlx::query(
        r#"
        UPDATE trails
        SET geom = merged.geom,
            start_point = merged.start_point,
            end_point = merged.end_point,
            region_id = region.region_id,
            updated_at = now()
        FROM (
            SELECT
                ST_Multi(ST_Collect(geom ORDER BY sequence)) AS geom,
                ST_StartPoint((ARRAY_AGG(geom ORDER BY sequence))[1]) AS start_point,
                ST_EndPoint((ARRAY_AGG(geom ORDER BY sequence DESC))[1]) AS end_point
            FROM segments
            WHERE trail_id = $1
        ) AS merged
        LEFT JOIN LATERAL (
            SELECT id AS region_id
            FROM regions
            WHERE boundary IS NOT NULL
              AND merged.geom IS NOT NULL
              AND ST_Intersects(boundary, ST_PointOnSurface(merged.geom))
            ORDER BY admin_level DESC NULLS LAST, ST_Area(boundary) ASC
            LIMIT 1
        ) AS region ON TRUE
        WHERE trails.id = $1
        "#,
    )
    .bind(trail_id)
    .execute(&mut *tx)
    .await
    .context("updating trail geometry and region")?;

    sqlx::query(
        r#"
        INSERT INTO trail_lines (
            osm_relation_id,
            id,
            name,
            network,
            difficulty,
            hike_type,
            geom
        )
        SELECT
            osm_relation_id,
            id,
            name,
            network,
            difficulty,
            hike_type,
            geom::geometry(MultiLineString, 4326)
        FROM trails
        WHERE id = $1
          AND geom IS NOT NULL
        ON CONFLICT (osm_relation_id) DO UPDATE
        SET id = EXCLUDED.id,
            name = EXCLUDED.name,
            network = EXCLUDED.network,
            difficulty = EXCLUDED.difficulty,
            hike_type = EXCLUDED.hike_type,
            geom = EXCLUDED.geom
        "#,
    )
    .bind(trail_id)
    .execute(&mut *tx)
    .await
    .context("upserting martin trail_lines row")?;

    sqlx::query(
        r#"
        DELETE FROM trail_start_points
        WHERE id = $1
        "#,
    )
    .bind(trail_id)
    .execute(&mut *tx)
    .await
    .context("deleting prior martin trail_start_points row")?;

    sqlx::query(
        r#"
        INSERT INTO trail_start_points (
            osm_relation_id,
            id,
            name,
            geom
        )
        SELECT
            osm_relation_id,
            id,
            name,
            start_point::geometry(Point, 4326)
        FROM trails
        WHERE id = $1
          AND start_point IS NOT NULL
        "#,
    )
    .bind(trail_id)
    .execute(&mut *tx)
    .await
    .context("upserting martin trail_start_points row")?;

    sqlx::query(
        r#"
        DELETE FROM trail_end_points
        WHERE id = $1
        "#,
    )
    .bind(trail_id)
    .execute(&mut *tx)
    .await
    .context("deleting prior martin trail_end_points row")?;

    sqlx::query(
        r#"
        INSERT INTO trail_end_points (
            osm_relation_id,
            id,
            name,
            geom
        )
        SELECT
            osm_relation_id,
            id,
            name,
            end_point::geometry(Point, 4326)
        FROM trails
        WHERE id = $1
          AND end_point IS NOT NULL
        "#,
    )
    .bind(trail_id)
    .execute(&mut *tx)
    .await
    .context("upserting martin trail_end_points row")?;

    let surface_rows = surface_totals
        .into_iter()
        .map(|(surface_type, distance_m)| {
            let pct = if total_distance > 0.0 {
                (distance_m / total_distance) * 100.0
            } else {
                0.0
            };
            (trail_id, surface_type.to_string(), distance_m, pct)
        })
        .collect::<Vec<_>>();
    insert_surface_breakdown_batch(&mut tx, &surface_rows).await?;

    let nearby_waypoints = sqlx::query_as::<_, RawWaypointRow>(
        r#"
        SELECT osm_type::text AS osm_type, osm_id, tags, ST_AsBinary(geom) AS geom_wkb
        FROM raw_waypoints
        WHERE ST_DWithin(
            geom::geography,
            (SELECT geom::geography FROM trails WHERE id = $1),
            150
        )
        ORDER BY osm_id
        "#,
    )
    .bind(trail_id)
    .fetch_all(&mut *tx)
    .await
    .context("fetching nearby raw waypoints")?;

    let mut feature_slugs = BTreeSet::new();
    let mut waypoint_rows = Vec::with_capacity(nearby_waypoints.len());

    for waypoint in nearby_waypoints {
        let kind = infer_waypoint_kind(&waypoint.tags);
        let name = tag_str(&waypoint.tags, "name");
        let elevation_m = tag_str(&waypoint.tags, "ele").and_then(parse_elevation);

        if let Some(feature_slug) = feature_for_waypoint_kind(kind) {
            feature_slugs.insert(feature_slug);
        }

        waypoint_rows.push((
            Uuid::new_v4(),
            trail_id,
            normalized_osm_type(&waypoint.osm_type)
                .unwrap_or("N")
                .to_string(),
            waypoint.osm_id,
            kind.to_string(),
            name.map(str::to_owned),
            elevation_m,
            waypoint.geom_wkb,
        ));
    }
    insert_waypoints_batch(&mut tx, &waypoint_rows).await?;

    let feature_rows = feature_slugs
        .into_iter()
        .map(|feature_slug| (trail_id, feature_slug.to_string(), "derived".to_string()))
        .collect::<Vec<_>>();
    insert_trail_features_batch(&mut tx, &feature_rows).await?;

    let landcover_rows = landcover_segments
        .iter()
        .enumerate()
        .map(|(sequence, segment)| {
            (
                trail_id,
                sequence as i32,
                segment.start_m,
                segment.end_m,
                segment.landcover_code,
                segment.landcover_label.to_string(),
                segment.dem_confidence.to_string(),
            )
        })
        .collect::<Vec<_>>();
    insert_landcover_segments_batch(&mut tx, &landcover_rows).await?;

    let warning_rows = warnings
        .iter()
        .map(|warning| (Uuid::new_v4(), trail_id, warning.clone()))
        .collect::<Vec<_>>();
    insert_stitch_warnings_batch(&mut tx, &warning_rows).await?;

    let diagnostic_rows = result
        .diagnostic_points
        .iter()
        .map(|point| {
            (
                Uuid::new_v4(),
                trail_id,
                point.kind.as_str().to_string(),
                point.coord.0,
                point.coord.1,
            )
        })
        .collect::<Vec<_>>();
    insert_diagnostic_points_batch(&mut tx, &diagnostic_rows).await?;

    if let Some(profile) = &elevation_profile {
        sqlx::query(
            r#"
            INSERT INTO elevation_profiles (
                id,
                trail_id,
                min_elevation_m,
                max_elevation_m,
                profile_points
            )
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(trail_id)
        .bind(profile.min_elevation_m)
        .bind(profile.max_elevation_m)
        .bind(Json(&profile.profile_points))
        .execute(&mut *tx)
        .await
        .context("inserting elevation profile")?;

        let duration_with_elevation = estimate_duration_minutes(
            total_distance,
            profile.ascent_m,
            profile.descent_m,
            difficulty.as_deref(),
        )
        .map(|value| value as i32);

        sqlx::query(
            r#"
            UPDATE trails
            SET ascent_m = $2,
                descent_m = $3,
                estimated_duration_min = $4,
                updated_at = now()
            WHERE id = $1
            "#,
        )
        .bind(trail_id)
        .bind(profile.ascent_m)
        .bind(profile.descent_m)
        .bind(duration_with_elevation)
        .execute(&mut *tx)
        .await
        .context("updating trail elevation stats")?;
    } else {
        sqlx::query(
            r#"
            UPDATE trails
            SET ascent_m = NULL,
                descent_m = NULL,
                updated_at = now()
            WHERE id = $1
            "#,
        )
        .bind(trail_id)
        .execute(&mut *tx)
        .await
        .context("clearing trail elevation stats")?;
    }

    tx.commit().await.context("committing transaction")?;
    Ok(warnings)
}

fn prepare_relation_members(members: &[RawRelationMember]) -> Vec<PreparedRelationMember> {
    let mut prepared = Vec::new();

    for member in members {
        if !member.kind.eq_ignore_ascii_case("w") {
            continue;
        }

        prepared.push(PreparedRelationMember {
            way_id: member.ref_id,
            role: member.role.trim().to_string(),
        });
    }

    prepared
}

fn normalized_direction_role(role: &str) -> Option<String> {
    match role.trim().to_ascii_lowercase().as_str() {
        "forward" => Some("forward".to_string()),
        "backward" => Some("backward".to_string()),
        _ => None,
    }
}

fn unsupported_member_role(role: &str) -> Option<&str> {
    let trimmed = role.trim();
    if trimmed.is_empty() || normalized_direction_role(trimmed).is_some() {
        None
    } else {
        Some(trimmed)
    }
}

fn collect_relation_way_ids(
    relation_id: i64,
    members: &[RawRelationMember],
    relation_lookup: &HashMap<i64, RawRelationRow>,
    stack: &mut Vec<i64>,
    warnings: &mut Vec<String>,
) -> Vec<i64> {
    let mut way_ids = Vec::new();

    for member in members {
        if member.kind.eq_ignore_ascii_case("w") {
            way_ids.push(member.ref_id);
            continue;
        }

        if !member.kind.eq_ignore_ascii_case("r") {
            continue;
        }

        if stack.contains(&member.ref_id) {
            warnings.push(format!(
                "nested relation {} references relation {} again -- cycle detected, skipped recursive expansion",
                relation_id, member.ref_id
            ));
            continue;
        }

        let Some(child_relation) = relation_lookup.get(&member.ref_id) else {
            warnings.push(format!(
                "nested relation member {} was not found in raw_relations -- skipped recursive expansion",
                member.ref_id
            ));
            continue;
        };

        stack.push(member.ref_id);
        way_ids.extend(collect_relation_way_ids(
            child_relation.osm_id,
            &child_relation.members.0,
            relation_lookup,
            stack,
            warnings,
        ));
        stack.pop();
    }

    way_ids
}

fn prepare_way_maps(
    rows: Vec<RawWayRow>,
) -> Result<(HashMap<i64, PreparedWay>, HashMap<i64, RawWayRow>)> {
    let mut prepared_by_id = HashMap::with_capacity(rows.len());
    let mut way_lookup = HashMap::with_capacity(rows.len());

    for row in rows {
        let geom = decode_wkb(&row.geom_wkb)?;
        let coords: Vec<(f64, f64)> = match geom {
            geo_types::Geometry::LineString(ls) => ls.points().map(|p| (p.x(), p.y())).collect(),
            other => anyhow::bail!("expected LineString for way {}, got {other:?}", row.osm_id),
        };

        let start = *coords.first().context("way has empty geometry")?;
        let end = *coords.last().context("way has empty geometry")?;

        prepared_by_id.insert(row.osm_id, PreparedWay { start, end });
        way_lookup.insert(row.osm_id, row);
    }

    Ok((prepared_by_id, way_lookup))
}

fn build_relation_way_records(
    relation_id: i64,
    members: &[RawRelationMember],
    relation_lookup: &HashMap<i64, RawRelationRow>,
    prepared_by_id: &HashMap<i64, PreparedWay>,
    stack: &mut Vec<i64>,
) -> Result<RelationBuild> {
    let prepared_members = prepare_relation_members(members);
    let mut direct_members = prepared_members.into_iter();
    let mut next_sequence = 0usize;
    let mut ways = Vec::new();
    let mut warnings = Vec::new();
    let mut missing_way_members = 0usize;
    let mut had_direction_roles = false;

    for member in members {
        if member.kind.eq_ignore_ascii_case("w") {
            let Some(prepared_member) = direct_members.next() else {
                continue;
            };
            if let Some(role) = unsupported_member_role(&prepared_member.role) {
                warnings.push(format!(
                    "member way {} uses unsupported role '{}' -- omitted from the main stitched route until appendix handling is implemented",
                    prepared_member.way_id, role
                ));
                continue;
            }
            let Some(prepared_way) = prepared_by_id.get(&prepared_member.way_id) else {
                missing_way_members += 1;
                continue;
            };
            let role = normalized_direction_role(&prepared_member.role).unwrap_or_default();
            if !role.is_empty() {
                had_direction_roles = true;
            }

            ways.push(WayRecord {
                osm_way_id: prepared_member.way_id,
                start: prepared_way.start,
                end: prepared_way.end,
                member_index: next_sequence,
                role,
            });
            next_sequence += 1;
            continue;
        }

        if !member.kind.eq_ignore_ascii_case("r") {
            continue;
        }

        if let Some(role) = unsupported_member_role(&member.role) {
            warnings.push(format!(
                "nested relation member {} uses unsupported role '{}' -- omitted from the main stitched route until appendix handling is implemented",
                member.ref_id, role
            ));
            continue;
        }

        if stack.contains(&member.ref_id) {
            warnings.push(format!(
                "nested relation {} references relation {} again -- cycle detected during route expansion",
                relation_id, member.ref_id
            ));
            continue;
        }

        let Some(child_relation) = relation_lookup.get(&member.ref_id) else {
            warnings.push(format!(
                "nested relation member {} was not found in raw_relations during route expansion",
                member.ref_id
            ));
            continue;
        };

        stack.push(member.ref_id);
        let child_build = build_relation_way_records(
            child_relation.osm_id,
            &child_relation.members.0,
            relation_lookup,
            prepared_by_id,
            stack,
        )?;
        stack.pop();

        let mut child_result = stitch_ways(&child_build.ways, child_build.had_direction_roles);
        warnings.extend(
            child_build
                .warnings
                .into_iter()
                .map(|warning| format!("nested relation {} -- {}", member.ref_id, warning)),
        );
        warnings.extend(
            child_result
                .warnings
                .drain(..)
                .map(|warning| format!("nested relation {} -- {}", member.ref_id, warning)),
        );
        missing_way_members += child_build.missing_way_members;
        had_direction_roles |=
            child_build.had_direction_roles || child_result.direction_source == "member_role";

        let mut ordered_child_ways = child_result.ordered_ways;
        match normalized_direction_role(&member.role).as_deref() {
            Some("backward") => {
                had_direction_roles = true;
                ordered_child_ways.reverse();
                for (_, reversed) in &mut ordered_child_ways {
                    *reversed = !*reversed;
                }
            }
            Some("forward") => {
                had_direction_roles = true;
            }
            _ => {}
        }

        for (way_id, reversed) in ordered_child_ways {
            let Some(prepared_way) = prepared_by_id.get(&way_id) else {
                missing_way_members += 1;
                continue;
            };

            let (start, end) = if reversed {
                (prepared_way.end, prepared_way.start)
            } else {
                (prepared_way.start, prepared_way.end)
            };

            ways.push(WayRecord {
                osm_way_id: way_id,
                start,
                end,
                member_index: next_sequence,
                role: String::new(),
            });
            next_sequence += 1;
        }
    }

    Ok(RelationBuild {
        ways,
        warnings,
        missing_way_members,
        had_direction_roles,
    })
}

async fn insert_segments_batch(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    rows: &[(
        Uuid,
        Uuid,
        i64,
        i32,
        Option<String>,
        Option<String>,
        Option<String>,
        f64,
        bool,
        Vec<u8>,
    )],
) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }

    let mut query = QueryBuilder::<Postgres>::new(
        r#"
        INSERT INTO segments (
            id,
            trail_id,
            osm_way_id,
            sequence,
            name,
            surface,
            highway_type,
            length_m,
            geom
        )
        "#,
    );

    query.push_values(rows, |mut builder, row| {
        builder
            .push_bind(row.0)
            .push_bind(row.1)
            .push_bind(row.2)
            .push_bind(row.3)
            .push_bind(&row.4)
            .push_bind(&row.5)
            .push_bind(&row.6)
            .push_bind(row.7);

        builder
            .push_unseparated(", CASE WHEN ")
            .push_bind_unseparated(row.8)
            .push_unseparated(" THEN ST_Reverse(ST_GeomFromWKB(")
            .push_bind_unseparated(&row.9)
            .push_unseparated(", 4326)) ELSE ST_GeomFromWKB(")
            .push_bind_unseparated(&row.9)
            .push_unseparated(", 4326) END");
    });

    query
        .build()
        .execute(&mut **tx)
        .await
        .context("inserting segments batch")?;

    Ok(())
}

async fn insert_surface_breakdown_batch(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    rows: &[(Uuid, String, f64, f64)],
) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }

    let mut query = QueryBuilder::<Postgres>::new(
        "INSERT INTO trail_surface_breakdown (trail_id, surface_type, distance_m, pct) ",
    );
    query.push_values(rows, |mut builder, row| {
        builder
            .push_bind(row.0)
            .push_bind(&row.1)
            .push_bind(row.2)
            .push_bind(row.3);
    });

    query
        .build()
        .execute(&mut **tx)
        .await
        .context("inserting trail surface breakdown batch")?;

    Ok(())
}

async fn insert_waypoints_batch(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    rows: &[(
        Uuid,
        Uuid,
        String,
        i64,
        String,
        Option<String>,
        Option<f64>,
        Vec<u8>,
    )],
) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }

    let mut query = QueryBuilder::<Postgres>::new(
        r#"
        INSERT INTO waypoints (
            id, trail_id, osm_type, osm_id, osm_node_id, type, name, elevation_m, geom
        )
        "#,
    );
    query.push_values(rows, |mut builder, row| {
        builder
            .push_bind(row.0)
            .push_bind(row.1)
            .push_bind(&row.2)
            .push_bind(row.3)
            .push_bind(if row.2 == "N" { Some(row.3) } else { None })
            .push_bind(&row.4)
            .push_bind(&row.5)
            .push_bind(row.6);

        builder
            .push_unseparated(", ST_GeomFromWKB(")
            .push_bind_unseparated(&row.7)
            .push_unseparated(", 4326)");
    });

    query
        .build()
        .execute(&mut **tx)
        .await
        .context("inserting waypoints batch")?;

    Ok(())
}

async fn insert_trail_features_batch(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    rows: &[(Uuid, String, String)],
) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }

    let mut query = QueryBuilder::<Postgres>::new(
        "INSERT INTO trail_features (trail_id, feature_slug, source) ",
    );
    query.push_values(rows, |mut builder, row| {
        builder.push_bind(row.0).push_bind(&row.1).push_bind(&row.2);
    });

    query
        .build()
        .execute(&mut **tx)
        .await
        .context("inserting trail features batch")?;

    Ok(())
}

async fn insert_landcover_segments_batch(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    rows: &[(Uuid, i32, f64, f64, i32, String, String)],
) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }

    let mut query = QueryBuilder::<Postgres>::new(
        r#"
        INSERT INTO trail_landcover_segments (
            trail_id,
            sequence,
            start_m,
            end_m,
            landcover_code,
            landcover_label,
            dem_confidence
        )
        "#,
    );
    query.push_values(rows, |mut builder, row| {
        builder
            .push_bind(row.0)
            .push_bind(row.1)
            .push_bind(row.2)
            .push_bind(row.3)
            .push_bind(row.4)
            .push_bind(&row.5)
            .push_bind(&row.6);
    });

    query
        .build()
        .execute(&mut **tx)
        .await
        .context("inserting landcover segments batch")?;

    Ok(())
}

async fn insert_stitch_warnings_batch(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    rows: &[(Uuid, Uuid, String)],
) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }

    let mut query =
        QueryBuilder::<Postgres>::new("INSERT INTO stitch_warnings (id, trail_id, message) ");
    query.push_values(rows, |mut builder, row| {
        builder.push_bind(row.0).push_bind(row.1).push_bind(&row.2);
    });

    query
        .build()
        .execute(&mut **tx)
        .await
        .context("inserting stitch warnings batch")?;

    Ok(())
}

async fn insert_diagnostic_points_batch(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    rows: &[(Uuid, Uuid, String, f64, f64)],
) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }

    let mut query = QueryBuilder::<Postgres>::new(
        r#"
        INSERT INTO stitch_diagnostic_points (id, trail_id, kind, geom)
        "#,
    );
    query.push_values(rows, |mut builder, row| {
        builder.push_bind(row.0).push_bind(row.1).push_bind(&row.2);

        builder
            .push_unseparated(", ST_SetSRID(ST_MakePoint(")
            .push_bind_unseparated(row.3)
            .push_unseparated(", ")
            .push_bind_unseparated(row.4)
            .push_unseparated("), 4326)");
    });

    query
        .build()
        .execute(&mut **tx)
        .await
        .context("inserting stitch diagnostic points batch")?;

    Ok(())
}

fn tag_str<'a>(tags: &'a Value, key: &str) -> Option<&'a str> {
    tags.get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
}

fn normalized_osm_type(value: &str) -> Option<&'static str> {
    match value.trim() {
        "N" => Some("N"),
        "W" => Some("W"),
        "R" => Some("R"),
        _ => None,
    }
}

fn osm_type_name(value: &str) -> Option<&'static str> {
    match normalized_osm_type(value)? {
        "N" => Some("node"),
        "W" => Some("way"),
        "R" => Some("relation"),
        _ => None,
    }
}

fn parse_admin_level_tag(tags: &Value) -> Option<i16> {
    tag_str(tags, "admin_level").and_then(|value| value.parse::<i16>().ok())
}

fn extract_country_code(tags: &Value) -> Option<String> {
    tag_str(tags, "ISO3166-1:alpha2")
        .and_then(normalize_country_code)
        .or_else(|| {
            tag_str(tags, "ISO3166-2")
                .and_then(|value| value.split('-').next())
                .and_then(normalize_country_code)
        })
        .or_else(|| tag_str(tags, "is_in:country_code").and_then(normalize_country_code))
        .or_else(|| tag_str(tags, "addr:country").and_then(normalize_country_code))
}

fn normalize_country_code(raw: &str) -> Option<String> {
    let normalized = raw.trim().to_ascii_uppercase();
    if normalized.len() == 2 && normalized.chars().all(|ch| ch.is_ascii_alphabetic()) {
        Some(normalized)
    } else {
        None
    }
}

fn derive_difficulty(tags: &Value, ways: &HashMap<i64, RawWayRow>) -> Option<String> {
    if let Some(value) = tag_str(tags, "sac_scale") {
        return Some(value.to_owned());
    }

    ways.values()
        .filter_map(|way| tag_str(&way.tags, "sac_scale"))
        .max_by_key(|value| sac_scale_rank(value))
        .map(ToOwned::to_owned)
}

fn sac_scale_rank(value: &str) -> usize {
    match value {
        "hiking" => 1,
        "mountain_hiking" => 2,
        "demanding_mountain_hiking" => 3,
        "alpine_hiking" => 4,
        "demanding_alpine_hiking" => 5,
        "difficult_alpine_hiking" => 6,
        _ => 0,
    }
}

fn estimate_duration_minutes(
    distance_m: f64,
    ascent_m: f64,
    descent_m: f64,
    difficulty: Option<&str>,
) -> Option<u32> {
    if distance_m <= 0.0 {
        return None;
    }

    let (km_per_hour, ascent_rate_m_per_hour, descent_rate_m_per_hour) = match difficulty {
        Some("difficult_alpine_hiking") => (1.8, 320.0, 700.0),
        Some("demanding_alpine_hiking") => (2.2, 380.0, 780.0),
        Some("alpine_hiking") => (2.8, 430.0, 840.0),
        Some("demanding_mountain_hiking") => (3.2, 500.0, 920.0),
        Some("mountain_hiking") => (3.8, 560.0, 980.0),
        _ => (4.5, 620.0, 1050.0),
    };

    let horizontal_hours = (distance_m / 1000.0) / km_per_hour;
    let ascent_hours = ascent_m.max(0.0) / ascent_rate_m_per_hour;
    let descent_hours = descent_m.max(0.0) / descent_rate_m_per_hour;
    let hours = horizontal_hours + ascent_hours + descent_hours;
    Some((hours * 60.0).round() as u32)
}

fn normalize_surface(surface: Option<&str>, highway_type: Option<&str>) -> &'static str {
    match surface.unwrap_or_default() {
        "asphalt" | "concrete" | "paved" | "paving_stones" | "sett" => "paved",
        "gravel" | "fine_gravel" | "compacted" | "pebblestone" => "gravel",
        "rock" | "stone" | "boulder" | "scree" => "rock",
        "ground" | "dirt" | "earth" | "soil" | "mud" | "grass" | "sand" | "clay" => "ground",
        "snow" | "ice" => "snow_ice",
        _ => match highway_type.unwrap_or_default() {
            "residential" => "paved",
            _ => "unknown",
        },
    }
}

fn infer_waypoint_kind(tags: &Value) -> &'static str {
    if let Some(value) = tag_str(tags, "emergency") {
        return match value {
            "phone" => "emergency_phone",
            "first_aid_kit" => "first_aid_kit",
            "defibrillator" => "defibrillator",
            "mountain_rescue" => "mountain_rescue",
            _ => "poi",
        };
    }

    if let Some(value) = tag_str(tags, "amenity") {
        return match value {
            "shelter" => match tag_str(tags, "shelter_type") {
                Some("basic_hut") => "basic_hut",
                Some("weather_shelter") => "weather_shelter",
                Some("lean_to") => "lean_to",
                Some("rock_shelter") => "rock_shelter",
                _ => "shelter",
            },
            "drinking_water" => "drinking_water",
            "toilets" => "toilets",
            "parking" => "parking",
            "ranger_station" => "ranger_station",
            "hospital" => "hospital",
            "clinic" => "clinic",
            _ => "poi",
        };
    }

    if tag_str(tags, "highway") == Some("trailhead") {
        return "trailhead";
    }

    if let Some(value) = tag_str(tags, "tourism") {
        return match value {
            "alpine_hut" => "alpine_hut",
            "wilderness_hut" => "wilderness_hut",
            "camp_site" => "camp_site",
            "viewpoint" => "viewpoint",
            "picnic_site" => "picnic_site",
            "information" => match tag_str(tags, "information") {
                Some("guidepost") => "guidepost",
                Some("map") => "information_map",
                Some("board") => "information_board",
                _ => "information",
            },
            _ => "poi",
        };
    }

    if let Some(value) = tag_str(tags, "natural") {
        return match value {
            "peak" => "peak",
            "spring" => "spring",
            "waterfall" => "waterfall",
            "cave_entrance" => "cave_entrance",
            "saddle" => "saddle",
            _ => "poi",
        };
    }

    if tag_str(tags, "mountain_pass") == Some("yes") {
        return "mountain_pass";
    }

    if tag_str(tags, "aerialway") == Some("station") {
        return "aerialway_station";
    }

    "poi"
}

fn dedupe_preserve_order(items: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::with_capacity(items.len());

    for item in items {
        if seen.insert(item.clone()) {
            deduped.push(item);
        }
    }

    deduped
}

fn feature_for_waypoint_kind(kind: &str) -> Option<&'static str> {
    match kind {
        "camp_site" => Some("camping"),
        "alpine_hut" | "wilderness_hut" | "basic_hut" | "weather_shelter" | "lean_to"
        | "rock_shelter" | "shelter" => Some("huts"),
        "peak" => Some("summit"),
        "spring" => Some("spring"),
        "drinking_water" => Some("drinking_water"),
        "mountain_rescue" | "ranger_station" | "emergency_phone" | "first_aid_kit"
        | "defibrillator" | "hospital" | "clinic" => Some("emergency"),
        "trailhead" => Some("trailhead"),
        "viewpoint" => Some("viewpoint"),
        "waterfall" => Some("waterfall"),
        "cave_entrance" => Some("cave"),
        "saddle" | "mountain_pass" => Some("pass"),
        _ => None,
    }
}

fn parse_elevation(raw: &str) -> Option<f64> {
    let normalized = raw.trim().to_ascii_lowercase().replace(',', "");
    let (number, multiplier) = if let Some(value) = normalized
        .strip_suffix("meters")
        .or_else(|| normalized.strip_suffix("metres"))
        .or_else(|| normalized.strip_suffix("meter"))
        .or_else(|| normalized.strip_suffix("metre"))
        .or_else(|| normalized.strip_suffix('m'))
    {
        (value, 1.0)
    } else if let Some(value) = normalized
        .strip_suffix("feet")
        .or_else(|| normalized.strip_suffix("foot"))
        .or_else(|| normalized.strip_suffix("ft"))
    {
        (value, 0.3048)
    } else {
        (normalized.as_str(), 1.0)
    };

    let elevation_m = number.trim().trim_start_matches('~').parse::<f64>().ok()? * multiplier;
    if elevation_m.is_finite() && (-500.0..=9000.0).contains(&elevation_m) {
        Some(elevation_m)
    } else {
        None
    }
}

impl ElevationSampler {
    fn from_env() -> Result<Option<Self>> {
        let mut dem_paths = env_path_list("DEM_PATHS", "DEM_PATH")?;
        if dem_paths.is_empty() {
            dem_paths = discover_raster_tiles(Path::new("data/dem_tiles"))?;
            if !dem_paths.is_empty() {
                tracing::info!(
                    dem_tiles = dem_paths.len(),
                    directory = "data/dem_tiles",
                    "DEM_PATH / DEM_PATHS not set -- auto-detected local DEM tiles"
                );
            }
        }
        if dem_paths.is_empty() {
            return Ok(None);
        }

        let gdal_capabilities = detect_gdal_capabilities()?;
        let dem_sources = prioritize_raster_sources(load_raster_sources(&dem_paths)?);

        tracing::info!(
            dem_sources = dem_sources
                .iter()
                .map(|source| format!(
                    "{}{}",
                    source.path.display(),
                    source
                        .nominal_resolution_m
                        .map(|resolution| format!(" ({resolution:.1}m)"))
                        .unwrap_or_default()
                ))
                .collect::<Vec<_>>()
                .join(", "),
            "prepared DEM source order"
        );

        Ok(Some(Self {
            dem_sources,
            gdal_capabilities,
            sample_spacing_m: env_f64(
                "ELEVATION_SAMPLE_SPACING_M",
                DEFAULT_ELEVATION_SAMPLE_SPACING_M,
            )?,
            smoothing_window: env_usize(
                "ELEVATION_SMOOTHING_WINDOW_SAMPLES",
                DEFAULT_ELEVATION_SMOOTHING_WINDOW,
            )?,
            gain_loss_threshold_m: env_f64(
                "ELEVATION_GAIN_LOSS_THRESHOLD_M",
                DEFAULT_GAIN_LOSS_THRESHOLD_M,
            )?,
        }))
    }

    fn sample_route(&self, coords: &[(f64, f64)]) -> Result<ComputedElevationProfile> {
        if coords.len() < 2 {
            anyhow::bail!("route has fewer than 2 coordinates");
        }

        let sample_positions = build_sample_positions(coords, self.sample_spacing_m);
        let sample_coords = sample_positions
            .iter()
            .map(|(_, coord)| *coord)
            .collect::<Vec<_>>();
        let raw_samples = sample_raster_values(
            &self.dem_sources,
            &sample_coords,
            "bilinear",
            self.gdal_capabilities,
        )?;
        let interpolated_samples = interpolate_missing_samples(&raw_samples)
            .context("DEM sampling returned no usable elevation points")?;
        let smoothed_samples = smooth_elevations(&interpolated_samples, self.smoothing_window);

        let profile_points = sample_positions
            .iter()
            .zip(smoothed_samples.iter())
            .map(|((pos_m, _), ele_m)| ElevationPoint {
                pos_m: round_to(*pos_m, 1),
                ele_m: round_to(*ele_m, 1),
            })
            .collect::<Vec<_>>();

        let (ascent_m, descent_m) =
            compute_gain_loss(&smoothed_samples, self.gain_loss_threshold_m);

        let min_elevation_m = smoothed_samples
            .iter()
            .min_by(|a, b| a.total_cmp(b))
            .context("profile has no minimum elevation")?;
        let max_elevation_m = smoothed_samples
            .iter()
            .max_by(|a, b| a.total_cmp(b))
            .context("profile has no maximum elevation")?;

        Ok(ComputedElevationProfile {
            min_elevation_m: round_to(*min_elevation_m, 1),
            max_elevation_m: round_to(*max_elevation_m, 1),
            ascent_m,
            descent_m,
            profile_points,
        })
    }
}

impl LandcoverSampler {
    fn from_env() -> Result<Option<Self>> {
        let Some(raw_path) = std::env::var_os("WORLDCOVER_PATH") else {
            return Ok(None);
        };

        let worldcover_path = PathBuf::from(raw_path);
        if !worldcover_path.exists() {
            anyhow::bail!(
                "WORLDCOVER_PATH points to '{}' but that file does not exist",
                worldcover_path.display()
            );
        }

        Ok(Some(Self {
            worldcover_source: load_raster_source(&worldcover_path)?,
            gdal_capabilities: detect_gdal_capabilities()?,
        }))
    }

    fn segment_route(&self, coords: &[(f64, f64)]) -> Result<Vec<LandcoverSegment>> {
        if coords.len() < 2 {
            return Ok(Vec::new());
        }

        let sample_positions = build_sample_positions(coords, DEFAULT_ELEVATION_SAMPLE_SPACING_M);
        let sample_coords = sample_positions
            .iter()
            .map(|(_, coord)| *coord)
            .collect::<Vec<_>>();
        let raw_values = sample_raster_values(
            std::slice::from_ref(&self.worldcover_source),
            &sample_coords,
            "nearest",
            self.gdal_capabilities,
        )?;
        let coded_samples = sample_positions
            .into_iter()
            .zip(raw_values)
            .filter_map(|((pos_m, _), value)| value.map(|sample| (pos_m, sample.round() as i32)))
            .collect::<Vec<_>>();

        Ok(segment_landcover_samples(&coded_samples, 200.0))
    }
}

fn env_path_list(list_key: &str, single_key: &str) -> Result<Vec<PathBuf>> {
    let mut paths: Vec<(&str, PathBuf)> = Vec::new();

    if let Some(raw_list) = std::env::var_os(list_key) {
        for item in raw_list.to_string_lossy().split(',') {
            let trimmed = item.trim();
            if trimmed.is_empty() {
                continue;
            }
            paths.push((list_key, PathBuf::from(trimmed)));
        }
    }

    if let Some(raw_path) = std::env::var_os(single_key) {
        let path = PathBuf::from(raw_path);
        if !path.as_os_str().is_empty() {
            paths.push((single_key, path));
        }
    }

    let mut deduped = Vec::new();
    let mut seen = HashSet::new();
    for (source_key, path) in paths {
        if !path.exists() {
            anyhow::bail!(
                "{} points to '{}' but that file does not exist",
                source_key,
                path.display()
            );
        }

        let key = path.to_string_lossy().to_string();
        if seen.insert(key) {
            deduped.push(path);
        }
    }

    Ok(deduped)
}

fn discover_raster_tiles(directory: &Path) -> Result<Vec<PathBuf>> {
    if !directory.exists() {
        return Ok(Vec::new());
    }

    let mut paths = fs::read_dir(directory)
        .with_context(|| format!("reading raster directory '{}'", directory.display()))?
        .filter_map(|entry| entry.ok().map(|item| item.path()))
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| matches!(ext.to_ascii_lowercase().as_str(), "tif" | "tiff"))
        })
        .collect::<Vec<_>>();

    paths.sort();
    Ok(paths)
}

fn env_f64(key: &str, default: f64) -> Result<f64> {
    match std::env::var(key) {
        Ok(value) if !value.trim().is_empty() => value
            .trim()
            .parse::<f64>()
            .with_context(|| format!("parsing {} as f64", key)),
        _ => Ok(default),
    }
}

fn env_usize(key: &str, default: usize) -> Result<usize> {
    match std::env::var(key) {
        Ok(value) if !value.trim().is_empty() => value
            .trim()
            .parse::<usize>()
            .with_context(|| format!("parsing {} as usize", key)),
        _ => Ok(default),
    }
}

fn detect_gdal_capabilities() -> Result<GdalCapabilities> {
    static GDAL_CAPABILITIES: OnceLock<std::result::Result<GdalCapabilities, String>> =
        OnceLock::new();

    match GDAL_CAPABILITIES.get_or_init(|| probe_gdal_capabilities().map_err(|err| err.to_string()))
    {
        Ok(capabilities) => Ok(*capabilities),
        Err(message) => Err(anyhow::anyhow!(message.clone())),
    }
}

fn probe_gdal_capabilities() -> Result<GdalCapabilities> {
    let output = Command::new("gdalinfo")
        .arg("--version")
        .output()
        .context("running gdalinfo --version")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("gdalinfo --version failed: {}", stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let version = parse_gdal_version(&stdout)
        .with_context(|| format!("parsing GDAL version from '{}'", stdout.trim()))?;

    let capabilities = GdalCapabilities {
        supports_batch_queries: version >= (3, 9),
        supports_interpolation_flag: version >= (3, 10),
    };

    if !capabilities.supports_batch_queries {
        tracing::warn!(
            gdal_version = stdout.trim(),
            "GDAL < 3.9 lacks batched gdallocationinfo queries; elevation imports will be much slower. Upgrade to GDAL 3.10 or newer."
        );
    } else if !capabilities.supports_interpolation_flag {
        tracing::warn!(
            gdal_version = stdout.trim(),
            "GDAL does not support gdallocationinfo -r; raster sampling will use nearest-neighbor lookups"
        );
    }

    Ok(capabilities)
}

fn parse_gdal_version(raw: &str) -> Result<(u32, u32)> {
    let token = raw
        .split_whitespace()
        .find(|part| part.chars().next().is_some_and(|ch| ch.is_ascii_digit()))
        .context("missing version token")?;

    let mut parts = token.split('.');
    let major = parts
        .next()
        .context("missing GDAL major version")?
        .parse::<u32>()
        .context("parsing GDAL major version")?;
    let minor_digits = parts
        .next()
        .context("missing GDAL minor version")?
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    let minor = minor_digits
        .parse::<u32>()
        .context("parsing GDAL minor version")?;

    Ok((major, minor))
}

fn load_raster_sources(paths: &[PathBuf]) -> Result<Vec<RasterSource>> {
    paths.iter().map(load_raster_source).collect()
}

fn load_raster_source(path: &PathBuf) -> Result<RasterSource> {
    let output = Command::new("gdalinfo")
        .arg("-json")
        .arg(path)
        .output()
        .with_context(|| format!("running gdalinfo -json against '{}'", path.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "gdalinfo -json failed for '{}': {}",
            path.display(),
            stderr.trim()
        );
    }

    let json: Value = serde_json::from_slice(&output.stdout)
        .with_context(|| format!("parsing gdalinfo JSON for '{}'", path.display()))?;
    let nodata_value = json
        .get("bands")
        .and_then(Value::as_array)
        .and_then(|bands| bands.first())
        .and_then(|band| band.get("noDataValue"))
        .and_then(Value::as_f64);
    let bounds_wgs84 = parse_wgs84_bounds(&json);
    let nominal_resolution_m = estimate_nominal_resolution_m(&json, bounds_wgs84);

    Ok(RasterSource {
        path: path.clone(),
        nodata_value,
        bounds_wgs84,
        nominal_resolution_m,
    })
}

fn prioritize_raster_sources(mut sources: Vec<RasterSource>) -> Vec<RasterSource> {
    sources.sort_by(
        |left, right| match (left.nominal_resolution_m, right.nominal_resolution_m) {
            (Some(a), Some(b)) => a.total_cmp(&b).then_with(|| left.path.cmp(&right.path)),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => left.path.cmp(&right.path),
        },
    );
    sources
}

fn parse_wgs84_bounds(json: &Value) -> Option<RasterBounds> {
    let rings = json
        .get("wgs84Extent")
        .and_then(|extent| extent.get("coordinates"))
        .and_then(Value::as_array)?;

    let mut min_lon = f64::INFINITY;
    let mut min_lat = f64::INFINITY;
    let mut max_lon = f64::NEG_INFINITY;
    let mut max_lat = f64::NEG_INFINITY;
    let mut saw_point = false;

    for ring in rings {
        let Some(points) = ring.as_array() else {
            continue;
        };
        for point in points {
            let Some(coords) = point.as_array() else {
                continue;
            };
            let Some(lon) = coords.first().and_then(Value::as_f64) else {
                continue;
            };
            let Some(lat) = coords.get(1).and_then(Value::as_f64) else {
                continue;
            };
            min_lon = min_lon.min(lon);
            min_lat = min_lat.min(lat);
            max_lon = max_lon.max(lon);
            max_lat = max_lat.max(lat);
            saw_point = true;
        }
    }

    if !saw_point {
        return None;
    }

    Some(RasterBounds {
        min_lon,
        min_lat,
        max_lon,
        max_lat,
    })
}

fn estimate_nominal_resolution_m(json: &Value, bounds: Option<RasterBounds>) -> Option<f64> {
    let bounds = bounds?;
    let size = json.get("size")?.as_array()?;
    let width = size.first().and_then(Value::as_u64)? as f64;
    let height = size.get(1).and_then(Value::as_u64)? as f64;
    if width <= 0.0 || height <= 0.0 {
        return None;
    }

    let center_lat_rad = ((bounds.min_lat + bounds.max_lat) / 2.0).to_radians();
    let lon_m = (bounds.max_lon - bounds.min_lon).abs() * 111_320.0 * center_lat_rad.cos().abs();
    let lat_m = (bounds.max_lat - bounds.min_lat).abs() * 110_574.0;
    let x_res_m = lon_m / width;
    let y_res_m = lat_m / height;
    let nominal = x_res_m.max(y_res_m);

    if nominal.is_finite() && nominal > 0.0 {
        Some(nominal)
    } else {
        None
    }
}

fn coord_in_bounds(coord: (f64, f64), bounds: RasterBounds) -> bool {
    let tolerance = 1e-9;
    coord.0 >= bounds.min_lon - tolerance
        && coord.0 <= bounds.max_lon + tolerance
        && coord.1 >= bounds.min_lat - tolerance
        && coord.1 <= bounds.max_lat + tolerance
}

fn ordered_route_coordinates(
    ordered_ways: &[(i64, bool)],
    way_lookup: &HashMap<i64, RawWayRow>,
) -> Result<Vec<(f64, f64)>> {
    let mut route = Vec::new();

    for (index, (way_id, reversed)) in ordered_ways.iter().enumerate() {
        let way = way_lookup
            .get(way_id)
            .with_context(|| format!("ordered way {} missing from way lookup", way_id))?;
        let geom = decode_wkb(&way.geom_wkb)?;
        let mut coords: Vec<(f64, f64)> = match geom {
            geo_types::Geometry::LineString(ls) => ls.points().map(|p| (p.x(), p.y())).collect(),
            other => anyhow::bail!("expected LineString for way {}, got {other:?}", way.osm_id),
        };

        if *reversed {
            coords.reverse();
        }

        if index > 0 && !route.is_empty() && !coords.is_empty() {
            let route_end = *route.last().expect("route has last coordinate");
            if approx_distance_m(route_end, coords[0]) <= 3.0 {
                coords.remove(0);
            }
        }

        route.extend(coords);
    }

    if route.len() < 2 {
        anyhow::bail!("ordered route has fewer than 2 coordinates");
    }

    Ok(route)
}

fn build_sample_positions(coords: &[(f64, f64)], interval_m: f64) -> Vec<(f64, (f64, f64))> {
    let mut cumulative = Vec::with_capacity(coords.len());
    let mut total_distance = 0.0;
    cumulative.push(0.0);

    for pair in coords.windows(2) {
        total_distance += approx_distance_m(pair[0], pair[1]);
        cumulative.push(total_distance);
    }

    if total_distance <= 0.0 {
        return vec![
            (0.0, coords[0]),
            (1.0, *coords.last().unwrap_or(&coords[0])),
        ];
    }

    let raw_samples = (total_distance / interval_m.max(1.0)).ceil() as usize + 1;
    let sample_count = raw_samples.max(MIN_ELEVATION_SAMPLES);
    let mut samples = Vec::with_capacity(sample_count);

    for sample_index in 0..sample_count {
        let fraction = if sample_count <= 1 {
            0.0
        } else {
            sample_index as f64 / (sample_count - 1) as f64
        };
        let target_distance = total_distance * fraction;
        let coord = interpolate_point_at_distance(coords, &cumulative, target_distance);
        samples.push((target_distance, coord));
    }

    samples
}

fn interpolate_point_at_distance(
    coords: &[(f64, f64)],
    cumulative: &[f64],
    target_distance: f64,
) -> (f64, f64) {
    if target_distance <= 0.0 {
        return coords[0];
    }

    let Some(&total_distance) = cumulative.last() else {
        return coords[0];
    };
    if target_distance >= total_distance {
        return *coords.last().unwrap_or(&coords[0]);
    }

    for segment_index in 1..cumulative.len() {
        if cumulative[segment_index] < target_distance {
            continue;
        }

        let segment_start = coords[segment_index - 1];
        let segment_end = coords[segment_index];
        let segment_distance = cumulative[segment_index] - cumulative[segment_index - 1];
        if segment_distance <= 0.0 {
            return segment_end;
        }

        let local_fraction = (target_distance - cumulative[segment_index - 1]) / segment_distance;
        return (
            segment_start.0 + (segment_end.0 - segment_start.0) * local_fraction,
            segment_start.1 + (segment_end.1 - segment_start.1) * local_fraction,
        );
    }

    *coords.last().unwrap_or(&coords[0])
}

fn sample_raster_values(
    sources: &[RasterSource],
    coords: &[(f64, f64)],
    interpolation: &str,
    gdal_capabilities: GdalCapabilities,
) -> Result<Vec<Option<f64>>> {
    if coords.is_empty() {
        return Ok(Vec::new());
    }

    if !gdal_capabilities.supports_batch_queries {
        return sample_raster_values_pointwise(sources, coords, interpolation, gdal_capabilities);
    }

    let mut merged = vec![None; coords.len()];

    for source in sources {
        if merged.iter().all(Option::is_some) {
            break;
        }

        let path_samples =
            sample_raster_values_for_path(source, coords, interpolation, gdal_capabilities)?;
        for (slot, sample) in merged.iter_mut().zip(path_samples) {
            if slot.is_none() {
                *slot = sample;
            }
        }
    }

    Ok(merged)
}

fn sample_raster_values_pointwise(
    sources: &[RasterSource],
    coords: &[(f64, f64)],
    interpolation: &str,
    gdal_capabilities: GdalCapabilities,
) -> Result<Vec<Option<f64>>> {
    let mut values = Vec::with_capacity(coords.len());

    for &coord in coords {
        let mut resolved = None;
        for source in sources {
            let value =
                sample_single_raster_value(source, coord, interpolation, gdal_capabilities)?;
            if value.is_some() {
                resolved = value;
                break;
            }
        }
        values.push(resolved);
    }

    Ok(values)
}

fn sample_raster_values_for_path(
    source: &RasterSource,
    coords: &[(f64, f64)],
    interpolation: &str,
    gdal_capabilities: GdalCapabilities,
) -> Result<Vec<Option<f64>>> {
    let applicable_indexes = coords
        .iter()
        .enumerate()
        .filter_map(|(index, coord)| {
            if source
                .bounds_wgs84
                .is_some_and(|bounds| !coord_in_bounds(*coord, bounds))
            {
                None
            } else {
                Some(index)
            }
        })
        .collect::<Vec<_>>();

    if applicable_indexes.is_empty() {
        return Ok(vec![None; coords.len()]);
    }

    let mut command = Command::new("gdallocationinfo");
    command
        .arg("-valonly")
        .arg("-E")
        .arg("-field_sep")
        .arg("\t")
        .arg("-ignore_extra_input")
        .arg("-wgs84");
    if gdal_capabilities.supports_interpolation_flag {
        command.arg("-r").arg(interpolation);
    }
    command
        .arg(&source.path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn().with_context(|| {
        format!(
            "starting gdallocationinfo against '{}'",
            source.path.display()
        )
    })?;

    {
        let mut stdin = child
            .stdin
            .take()
            .context("failed to open gdallocationinfo stdin")?;
        for &coord_index in &applicable_indexes {
            let (lon, lat) = coords[coord_index];
            writeln!(stdin, "{lon} {lat}").with_context(|| {
                format!("writing coordinate batch for '{}'", source.path.display())
            })?;
        }
    }

    let output = child.wait_with_output().with_context(|| {
        format!(
            "running gdallocationinfo against '{}'",
            source.path.display()
        )
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Ok(values) =
        parse_gdallocationinfo_values(&stdout, applicable_indexes.len(), source.nodata_value)
    {
        let mut resolved = vec![None; coords.len()];
        for (offset, value) in values.into_iter().enumerate() {
            resolved[applicable_indexes[offset]] = value;
        }
        return Ok(resolved);
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        anyhow::bail!(
            "gdallocationinfo failed for '{}' while sampling {} point(s): {}",
            source.path.display(),
            coords.len(),
            stderr.trim()
        );
    }

    let values =
        parse_gdallocationinfo_values(&stdout, applicable_indexes.len(), source.nodata_value)
            .with_context(|| {
                format!(
                    "parsing gdallocationinfo output from '{}'",
                    source.path.display()
                )
            })?;
    let mut resolved = vec![None; coords.len()];
    for (offset, value) in values.into_iter().enumerate() {
        resolved[applicable_indexes[offset]] = value;
    }
    Ok(resolved)
}

fn sample_single_raster_value(
    source: &RasterSource,
    coord: (f64, f64),
    interpolation: &str,
    gdal_capabilities: GdalCapabilities,
) -> Result<Option<f64>> {
    if source
        .bounds_wgs84
        .is_some_and(|bounds| !coord_in_bounds(coord, bounds))
    {
        return Ok(None);
    }

    let mut command = Command::new("gdallocationinfo");
    command.arg("-valonly").arg("-wgs84");
    if gdal_capabilities.supports_interpolation_flag {
        command.arg("-r").arg(interpolation);
    }
    command
        .arg(&source.path)
        .arg(coord.0.to_string())
        .arg(coord.1.to_string());

    let output = command.output().with_context(|| {
        format!(
            "running gdallocationinfo against '{}'",
            source.path.display()
        )
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Ok(values) = parse_gdallocationinfo_values(&stdout, 1, source.nodata_value) {
        return Ok(values.into_iter().next().flatten());
    }

    if !output.status.success() && stdout.trim().is_empty() && output.stderr.is_empty() {
        return Ok(None);
    }

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "gdallocationinfo failed for {:.6},{:.6} on '{}': {}",
            coord.0,
            coord.1,
            source.path.display(),
            stderr.trim()
        );
    }

    parse_gdallocationinfo_values(&stdout, 1, source.nodata_value)
        .with_context(|| {
            format!(
                "parsing gdallocationinfo output from '{}'",
                source.path.display()
            )
        })
        .map(|values| values.into_iter().next().flatten())
}

fn parse_gdallocationinfo_values(
    stdout: &str,
    expected_count: usize,
    nodata_value: Option<f64>,
) -> Result<Vec<Option<f64>>> {
    let lines = stdout.lines().collect::<Vec<_>>();
    if lines.len() != expected_count {
        anyhow::bail!(
            "expected {expected_count} sampled line(s), got {}",
            lines.len()
        );
    }

    let mut values = Vec::with_capacity(expected_count);
    for line in lines {
        let value_field = line.rsplit('\t').next().unwrap_or_default().trim();
        if value_field.is_empty() || value_field.eq_ignore_ascii_case("nan") {
            values.push(None);
            continue;
        }

        let parsed = value_field
            .parse::<f64>()
            .with_context(|| format!("invalid raster sample value '{value_field}'"))?;
        if !parsed.is_finite() || nodata_matches(parsed, nodata_value) {
            values.push(None);
        } else {
            values.push(Some(parsed));
        }
    }

    Ok(values)
}

fn nodata_matches(value: f64, nodata_value: Option<f64>) -> bool {
    let Some(nodata_value) = nodata_value else {
        return false;
    };

    let tolerance = 1e-9_f64.max(nodata_value.abs() * 1e-9);
    (value - nodata_value).abs() <= tolerance
}

fn interpolate_missing_samples(samples: &[Option<f64>]) -> Option<Vec<f64>> {
    let valid_indexes = samples
        .iter()
        .enumerate()
        .filter_map(|(index, value)| value.map(|_| index))
        .collect::<Vec<_>>();

    if valid_indexes.len() < 2 {
        return None;
    }

    let mut interpolated = samples
        .iter()
        .map(|value| value.unwrap_or(f64::NAN))
        .collect::<Vec<_>>();

    let first_valid = valid_indexes[0];
    let first_value = interpolated[first_valid];
    for slot in interpolated.iter_mut().take(first_valid) {
        *slot = first_value;
    }

    let last_valid = *valid_indexes.last().expect("has last valid index");
    let last_value = interpolated[last_valid];
    for slot in interpolated.iter_mut().skip(last_valid + 1) {
        *slot = last_value;
    }

    for window in valid_indexes.windows(2) {
        let left = window[0];
        let right = window[1];
        let left_value = interpolated[left];
        let right_value = interpolated[right];
        let span = (right - left) as f64;

        if span <= 1.0 {
            continue;
        }

        for index in (left + 1)..right {
            let fraction = (index - left) as f64 / span;
            interpolated[index] = left_value + (right_value - left_value) * fraction;
        }
    }

    Some(interpolated)
}

fn smooth_elevations(elevations: &[f64], window_samples: usize) -> Vec<f64> {
    if elevations.is_empty() {
        return Vec::new();
    }

    let mut window = window_samples.max(1);
    if window % 2 == 0 {
        window += 1;
    }
    if elevations.len() < window {
        return elevations.to_vec();
    }

    let pad = window / 2;
    let mut padded = Vec::with_capacity(elevations.len() + (pad * 2));
    padded.extend(std::iter::repeat_n(elevations[0], pad));
    padded.extend_from_slice(elevations);
    padded.extend(std::iter::repeat_n(
        *elevations.last().unwrap_or(&elevations[0]),
        pad,
    ));

    let mut smoothed = Vec::with_capacity(elevations.len());
    for start in 0..elevations.len() {
        let slice = &padded[start..(start + window)];
        smoothed.push(slice.iter().sum::<f64>() / window as f64);
    }

    smoothed
}

fn compute_gain_loss(elevations: &[f64], threshold_m: f64) -> (f64, f64) {
    if elevations.len() < 2 {
        return (0.0, 0.0);
    }

    let mut total_ascent = 0.0;
    let mut total_descent = 0.0;
    let mut reference = elevations[0];

    for value in elevations.iter().skip(1) {
        let diff = *value - reference;
        if diff >= threshold_m {
            total_ascent += diff;
            reference = *value;
        } else if diff <= -threshold_m {
            total_descent += -diff;
            reference = *value;
        }
    }

    (round_to(total_ascent, 1), round_to(total_descent, 1))
}

fn segment_landcover_samples(
    coded_samples: &[(f64, i32)],
    min_segment_m: f64,
) -> Vec<LandcoverSegment> {
    if coded_samples.is_empty() {
        return Vec::new();
    }

    let mut raw_segments = Vec::new();
    let mut start_index = 0usize;
    for index in 1..=coded_samples.len() {
        if index == coded_samples.len() || coded_samples[index].1 != coded_samples[start_index].1 {
            raw_segments.push((start_index, index - 1, coded_samples[start_index].1));
            start_index = index;
        }
    }

    let mut merged = raw_segments;
    let mut changed = true;
    while changed && merged.len() > 1 {
        changed = false;
        for index in 0..merged.len() {
            let (start, end, _) = merged[index];
            let length_m = coded_samples[end].0 - coded_samples[start].0;
            if length_m >= min_segment_m {
                continue;
            }

            let other = if index == 0 {
                1
            } else if index == merged.len() - 1 {
                index - 1
            } else {
                let left = segment_length_m(&merged[index - 1], coded_samples);
                let right = segment_length_m(&merged[index + 1], coded_samples);
                if left >= right {
                    index - 1
                } else {
                    index + 1
                }
            };

            let lo = index.min(other);
            let hi = index.max(other);
            let new_start = merged[lo].0;
            let new_end = merged[hi].1;
            let keep_code = merged[other].2;
            merged.splice(lo..=hi, [(new_start, new_end, keep_code)]);
            changed = true;
            break;
        }
    }

    merged
        .into_iter()
        .map(|(start, end, code)| {
            let (label, confidence) = worldcover_meta(code);
            LandcoverSegment {
                start_m: round_to(coded_samples[start].0, 1),
                end_m: round_to(coded_samples[end].0, 1),
                landcover_code: code,
                landcover_label: label,
                dem_confidence: confidence,
            }
        })
        .collect()
}

fn segment_length_m(segment: &(usize, usize, i32), coded_samples: &[(f64, i32)]) -> f64 {
    coded_samples[segment.1].0 - coded_samples[segment.0].0
}

fn worldcover_meta(code: i32) -> (&'static str, &'static str) {
    match code {
        10 => ("tree cover", "lower (canopy noise)"),
        20 => ("shrubland", "normal"),
        30 => ("grassland", "normal"),
        40 => ("cropland", "normal"),
        50 => ("built-up", "normal"),
        60 => ("bare / sparse vegetation", "normal"),
        70 => ("snow and ice", "lower (glacier/DEM epoch)"),
        80 => ("permanent water", "normal"),
        90 => ("herbaceous wetland", "normal"),
        95 => ("mangroves", "normal"),
        100 => ("moss and lichen", "normal"),
        _ => ("unknown", "unknown"),
    }
}

fn round_to(value: f64, decimals: i32) -> f64 {
    let factor = 10_f64.powi(decimals);
    (value * factor).round() / factor
}

pub(crate) fn approx_distance_m(a: (f64, f64), b: (f64, f64)) -> f64 {
    let lat_rad = a.1.to_radians();
    let m_per_deg_lat = 111_320.0;
    let m_per_deg_lon = 111_320.0 * lat_rad.cos();
    let dx = (a.0 - b.0) * m_per_deg_lon;
    let dy = (a.1 - b.1) * m_per_deg_lat;
    (dx * dx + dy * dy).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outdoor_waypoint_kinds_preserve_field_useful_semantics() {
        assert_eq!(
            infer_waypoint_kind(&serde_json::json!({ "amenity": "drinking_water" })),
            "drinking_water"
        );
        assert_eq!(
            infer_waypoint_kind(&serde_json::json!({ "natural": "spring" })),
            "spring"
        );
        assert_eq!(
            infer_waypoint_kind(&serde_json::json!({
                "amenity": "shelter",
                "shelter_type": "weather_shelter"
            })),
            "weather_shelter"
        );
        assert_eq!(
            infer_waypoint_kind(&serde_json::json!({
                "tourism": "information",
                "information": "guidepost"
            })),
            "guidepost"
        );
        assert_eq!(
            infer_waypoint_kind(&serde_json::json!({ "emergency": "phone" })),
            "emergency_phone"
        );
        assert_eq!(
            infer_waypoint_kind(&serde_json::json!({ "highway": "trailhead" })),
            "trailhead"
        );
    }

    #[test]
    fn original_mapper_manifest_accepts_only_version_one_authorship() {
        let manifest = parse_original_mapper_manifest(
            br#"{
                "schema_version": 1,
                "relations": [{
                    "osm_relation_id": 42,
                    "osm_relation_version": 1,
                    "display_name": "  Mapper  ",
                    "provider_user_id": " 7 ",
                    "profile_url": "https://www.openstreetmap.org/user/Mapper",
                    "mapped_at": "2024-01-15T10:30:00Z"
                }]
            }"#,
        )
        .expect("valid version-one manifest");

        assert_eq!(manifest.relations.len(), 1);
        assert_eq!(manifest.relations[0].display_name, "Mapper");
        assert_eq!(manifest.relations[0].provider_user_id.as_deref(), Some("7"));
    }

    #[test]
    fn original_mapper_manifest_rejects_later_relation_versions() {
        let error = parse_original_mapper_manifest(
            br#"{
                "schema_version": 1,
                "relations": [{
                    "osm_relation_id": 42,
                    "osm_relation_version": 2,
                    "display_name": "Latest editor",
                    "mapped_at": "2024-01-15T10:30:00Z"
                }]
            }"#,
        )
        .expect_err("version two must not be called the original mapper");

        assert!(error.to_string().contains("only relation version 1"));
    }

    #[test]
    fn original_mapper_manifest_rejects_duplicate_relations() {
        let error = parse_original_mapper_manifest(
            br#"{
                "schema_version": 1,
                "relations": [
                    {
                        "osm_relation_id": 42,
                        "osm_relation_version": 1,
                        "display_name": "Mapper",
                        "mapped_at": "2024-01-15T10:30:00Z"
                    },
                    {
                        "osm_relation_id": 42,
                        "osm_relation_version": 1,
                        "display_name": "Mapper",
                        "mapped_at": "2024-01-15T10:30:00Z"
                    }
                ]
            }"#,
        )
        .expect_err("duplicate relation ids must be rejected");

        assert!(error.to_string().contains("more than once"));
    }

    fn raw_way(osm_id: i64, geom_wkb: Vec<u8>) -> RawWayRow {
        RawWayRow {
            osm_id,
            tags: serde_json::json!({ "highway": "path" }),
            geom_wkb,
            length_m: 100.0,
        }
    }

    fn raw_relation(osm_id: i64, tags: Value, members: Vec<RawRelationMember>) -> RawRelationRow {
        RawRelationRow {
            osm_id,
            tags,
            members: Json(members),
        }
    }

    fn linestring_wkb(coords: &[(f64, f64)]) -> Vec<u8> {
        use geo_types::{Geometry, LineString};

        let geometry = Geometry::LineString(LineString::from(coords.to_vec()));
        wkb::geom_to_wkb(&geometry).expect("encode linestring")
    }

    #[test]
    fn build_relation_way_records_preserves_duplicate_member_occurrences() {
        let row = raw_way(42, linestring_wkb(&[(77.0, 32.0), (77.1, 32.1)]));
        let (prepared_by_id, lookup) = prepare_way_maps(vec![row]).expect("prepare way maps");
        let relation_lookup = HashMap::from([(
            100,
            raw_relation(
                100,
                serde_json::json!({ "name": "dup route" }),
                vec![
                    RawRelationMember {
                        kind: "w".to_string(),
                        ref_id: 42,
                        role: String::new(),
                    },
                    RawRelationMember {
                        kind: "w".to_string(),
                        ref_id: 42,
                        role: String::new(),
                    },
                ],
            ),
        )]);

        let build = build_relation_way_records(
            100,
            &relation_lookup.get(&100).expect("root relation").members.0,
            &relation_lookup,
            &prepared_by_id,
            &mut vec![100],
        )
        .expect("build relation ways");

        assert_eq!(lookup.len(), 1);
        assert_eq!(build.missing_way_members, 0);
        assert_eq!(build.ways.len(), 2);
        assert_eq!(build.ways[0].osm_way_id, 42);
        assert_eq!(build.ways[1].osm_way_id, 42);
        assert_eq!(build.ways[0].member_index, 0);
        assert_eq!(build.ways[1].member_index, 1);
    }

    #[test]
    fn nested_backward_relation_member_reverses_child_route_order() {
        let rows = vec![
            raw_way(1, linestring_wkb(&[(0.0, 0.0), (1.0, 0.0)])),
            raw_way(2, linestring_wkb(&[(1.0, 0.0), (2.0, 0.0)])),
        ];
        let (prepared_by_id, _) = prepare_way_maps(rows).expect("prepare way maps");
        let relation_lookup = HashMap::from([
            (
                200,
                raw_relation(
                    200,
                    serde_json::json!({ "name": "child route" }),
                    vec![
                        RawRelationMember {
                            kind: "w".to_string(),
                            ref_id: 1,
                            role: String::new(),
                        },
                        RawRelationMember {
                            kind: "w".to_string(),
                            ref_id: 2,
                            role: String::new(),
                        },
                    ],
                ),
            ),
            (
                201,
                raw_relation(
                    201,
                    serde_json::json!({ "name": "parent route" }),
                    vec![RawRelationMember {
                        kind: "r".to_string(),
                        ref_id: 200,
                        role: "backward".to_string(),
                    }],
                ),
            ),
        ]);

        let build = build_relation_way_records(
            201,
            &relation_lookup
                .get(&201)
                .expect("parent relation")
                .members
                .0,
            &relation_lookup,
            &prepared_by_id,
            &mut vec![201],
        )
        .expect("build nested relation ways");

        let ordered_ids = build
            .ways
            .iter()
            .map(|way| (way.osm_way_id, way.start, way.end))
            .collect::<Vec<_>>();

        assert_eq!(ordered_ids[0].0, 2);
        assert_eq!(ordered_ids[0].1, (2.0, 0.0));
        assert_eq!(ordered_ids[0].2, (1.0, 0.0));
        assert_eq!(ordered_ids[1].0, 1);
        assert_eq!(ordered_ids[1].1, (1.0, 0.0));
        assert_eq!(ordered_ids[1].2, (0.0, 0.0));
        assert!(build.had_direction_roles);
    }

    #[test]
    fn unsupported_way_role_is_omitted_from_main_route() {
        let rows = vec![
            raw_way(1, linestring_wkb(&[(0.0, 0.0), (1.0, 0.0)])),
            raw_way(2, linestring_wkb(&[(1.0, 0.0), (2.0, 0.0)])),
        ];
        let (prepared_by_id, _) = prepare_way_maps(rows).expect("prepare way maps");
        let relation_lookup = HashMap::from([(
            300,
            raw_relation(
                300,
                serde_json::json!({ "name": "appendix route" }),
                vec![
                    RawRelationMember {
                        kind: "w".to_string(),
                        ref_id: 1,
                        role: String::new(),
                    },
                    RawRelationMember {
                        kind: "w".to_string(),
                        ref_id: 2,
                        role: "alternative".to_string(),
                    },
                ],
            ),
        )]);

        let build = build_relation_way_records(
            300,
            &relation_lookup.get(&300).expect("root relation").members.0,
            &relation_lookup,
            &prepared_by_id,
            &mut vec![300],
        )
        .expect("build relation ways");

        assert_eq!(build.ways.len(), 1);
        assert_eq!(build.ways[0].osm_way_id, 1);
        assert!(build
            .warnings
            .iter()
            .any(|warning| warning.contains("unsupported role 'alternative'")));
    }

    #[test]
    fn dedupe_preserve_order_keeps_first_occurrence() {
        let items = vec![
            "branch point at a shared node".to_string(),
            "branch point at a shared node".to_string(),
            "missing member".to_string(),
            "branch point at a shared node".to_string(),
            "missing member".to_string(),
        ];

        let deduped = dedupe_preserve_order(items);

        assert_eq!(
            deduped,
            vec![
                "branch point at a shared node".to_string(),
                "missing member".to_string(),
            ]
        );
    }

    #[test]
    fn parse_elevation_handles_common_units() {
        assert_eq!(parse_elevation("6,932 m"), Some(6932.0));
        assert_eq!(parse_elevation("12000 ft"), Some(3657.6000000000004));
        assert_eq!(parse_elevation("~4200 metres"), Some(4200.0));
    }

    #[test]
    fn parse_elevation_rejects_ranges_and_impossible_values() {
        assert_eq!(parse_elevation("6353-6362"), None);
        assert_eq!(parse_elevation("99999"), None);
    }

    #[test]
    fn estimate_duration_respects_difficulty() {
        let easy = estimate_duration_minutes(10_000.0, 0.0, 0.0, Some("hiking")).unwrap();
        let alpine = estimate_duration_minutes(10_000.0, 0.0, 0.0, Some("alpine_hiking")).unwrap();

        assert!(alpine > easy);
    }

    #[test]
    fn estimate_duration_adds_elevation_penalty() {
        let flat = estimate_duration_minutes(10_000.0, 0.0, 0.0, Some("hiking")).unwrap();
        let climbing = estimate_duration_minutes(10_000.0, 900.0, 300.0, Some("hiking")).unwrap();

        assert!(climbing > flat);
    }

    #[test]
    fn compute_gain_loss_ignores_small_noise() {
        let elevations = vec![1000.0, 1004.0, 1002.0, 1007.0, 1003.0];
        let (ascent, descent) = compute_gain_loss(&elevations, 10.0);

        assert_eq!(ascent, 0.0);
        assert_eq!(descent, 0.0);
    }

    #[test]
    fn compute_gain_loss_counts_real_climbs_and_descents() {
        let elevations = vec![1000.0, 1014.0, 1028.0, 1010.0, 990.0];
        let (ascent, descent) = compute_gain_loss(&elevations, 10.0);

        assert!(ascent > 0.0);
        assert!(descent > 0.0);
    }

    #[test]
    fn segment_landcover_samples_merges_short_runs() {
        let samples = vec![
            (0.0, 10),
            (100.0, 10),
            (150.0, 30),
            (220.0, 10),
            (400.0, 10),
        ];

        let segments = segment_landcover_samples(&samples, 200.0);

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].landcover_code, 10);
        assert_eq!(segments[0].landcover_label, "tree cover");
    }

    #[test]
    fn build_sample_positions_keeps_long_routes_at_requested_resolution() {
        let coords = vec![(77.0, 32.0), (77.64, 32.0)];

        let samples = build_sample_positions(&coords, 30.0);

        assert!(samples.len() > 1_500);
        assert_eq!(samples.first().map(|sample| sample.0), Some(0.0));
        assert!(samples.last().map(|sample| sample.0).unwrap_or(0.0) > 50_000.0);
    }

    #[test]
    fn parse_gdallocationinfo_values_handles_missing_samples() {
        let stdout = "77.0\t32.0\t42\n76.0\t31.0\t\n77.2\t32.2\tnan\n";

        let parsed =
            parse_gdallocationinfo_values(stdout, 3, Some(-9999.0)).expect("parse samples");

        assert_eq!(parsed, vec![Some(42.0), None, None]);
    }

    #[test]
    fn parse_gdallocationinfo_values_treats_nodata_sentinel_as_missing() {
        let stdout = "77.0\t32.0\t42\n77.2\t32.2\t-9999\n";

        let parsed =
            parse_gdallocationinfo_values(stdout, 2, Some(-9999.0)).expect("parse samples");

        assert_eq!(parsed, vec![Some(42.0), None]);
    }

    #[test]
    fn parse_gdal_version_reads_major_and_minor() {
        let version =
            parse_gdal_version("GDAL 3.12.2 \"Chicoutimi\", released 2026/02/03").unwrap();

        assert_eq!(version, (3, 12));
    }

    #[test]
    fn nodata_matches_allows_small_rounding_error() {
        assert!(nodata_matches(-1234.5678904, Some(-1234.5678901)));
        assert!(!nodata_matches(-1234.56, Some(-1234.5678901)));
    }

    #[test]
    fn parse_wgs84_bounds_reads_extent_polygon() {
        let json = serde_json::json!({
            "wgs84Extent": {
                "coordinates": [[
                    [77.0, 32.0],
                    [78.0, 32.0],
                    [78.0, 33.0],
                    [77.0, 33.0],
                    [77.0, 32.0]
                ]]
            }
        });

        let bounds = parse_wgs84_bounds(&json).expect("parse bounds");

        assert_eq!(
            bounds,
            RasterBounds {
                min_lon: 77.0,
                min_lat: 32.0,
                max_lon: 78.0,
                max_lat: 33.0,
            }
        );
    }

    #[test]
    fn estimate_nominal_resolution_uses_wgs84_extent_and_size() {
        let json = serde_json::json!({
            "size": [3600, 3600]
        });
        let bounds = RasterBounds {
            min_lon: 77.0,
            min_lat: 32.0,
            max_lon: 78.0,
            max_lat: 33.0,
        };

        let resolution = estimate_nominal_resolution_m(&json, Some(bounds)).expect("resolution");

        assert!(resolution > 20.0);
        assert!(resolution < 40.0);
    }

    #[test]
    fn prioritize_raster_sources_prefers_finer_resolution() {
        let sources = vec![
            RasterSource {
                path: PathBuf::from("coarse.tif"),
                nodata_value: None,
                bounds_wgs84: None,
                nominal_resolution_m: Some(30.0),
            },
            RasterSource {
                path: PathBuf::from("fine.tif"),
                nodata_value: None,
                bounds_wgs84: None,
                nominal_resolution_m: Some(8.0),
            },
            RasterSource {
                path: PathBuf::from("unknown.tif"),
                nodata_value: None,
                bounds_wgs84: None,
                nominal_resolution_m: None,
            },
        ];

        let prioritized = prioritize_raster_sources(sources);
        let ordered = prioritized
            .iter()
            .map(|source| source.path.to_string_lossy().to_string())
            .collect::<Vec<_>>();

        assert_eq!(ordered, vec!["fine.tif", "coarse.tif", "unknown.tif"]);
    }

    #[test]
    fn coord_in_bounds_accepts_points_inside_extent() {
        let bounds = RasterBounds {
            min_lon: 77.0,
            min_lat: 32.0,
            max_lon: 78.0,
            max_lat: 33.0,
        };

        assert!(coord_in_bounds((77.4, 32.5), bounds));
        assert!(!coord_in_bounds((78.4, 32.5), bounds));
    }
}
