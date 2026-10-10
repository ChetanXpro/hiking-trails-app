use anyhow::{Context, Result};
use sqlx::PgPool;

pub async fn ensure_expected_schema(pool: &sqlx::PgPool) -> Result<()> {
    let version: i32 = sqlx::query_scalar(
        "SELECT version FROM schema_version ORDER BY applied_at DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .context("querying schema_version")?
    .unwrap_or(0);

    if version < 13 {
        anyhow::bail!(
            "expected schema version >= 13, got {version}. Run migrations with: cargo run --bin migrate",
        );
    }

    let tables = vec![
        "trails",
        "trail_segments",
        "trail_waypoints",
        "trail_surface_breakdown",
        "trail_features",
        "trail_landcover_breakdown",
        "trail_stitch_warnings",
        "trail_diagnostic_points",
        "regions",
        "places",
        "outdoor_waypoints",
    ];

    for table in tables {
        if !raw_table_exists(pool, table).await? {
            anyhow::bail!("required table '{table}' does not exist");
        }
    }

    Ok(())
}

pub async fn ensure_raw_osm_identity_column(pool: &sqlx::PgPool, table_name: &str) -> Result<()> {
    let column_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_name = $1 AND column_name = 'osm_id'
        )",
    )
    .bind(table_name)
    .fetch_one(pool)
    .await?;

    if !column_exists {
        anyhow::bail!(
            "table {table_name} does not have osm_id column; schema may be out of date",
        );
    }

    Ok(())
}

pub async fn raw_table_exists(pool: &sqlx::PgPool, table_name: &str) -> Result<bool> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM information_schema.tables
            WHERE table_schema = 'public' AND table_name = $1
        )",
    )
    .bind(table_name)
    .fetch_one(pool)
    .await?;
    Ok(exists)
}
