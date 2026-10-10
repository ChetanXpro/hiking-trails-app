use anyhow::{Context, Result};
use sqlx::PgPool;

pub async fn sync_regions_and_places(pool: &PgPool) -> Result<()> {
    sync_country_bounds(pool).await?;
    sync_regions(pool).await?;
    sync_places(pool).await?;
    sync_outdoor_waypoints(pool).await?;
    Ok(())
}

pub async fn sync_country_bounds(pool: &PgPool) -> Result<()> {
    tracing::info!("syncing country bounds from Natural Earth");

    sqlx::query(
        r#"
        INSERT INTO regions (name, parent_name, kind, geometry)
        SELECT name, NULL, 'country', ST_GeomFromGeoJSON(geojson)
        FROM (
            VALUES
            ('India', '{"type":"Polygon","coordinates":[[...]]}'),
            ('Nepal', '{"type":"Polygon","coordinates":[[...]]}'),
            ('Bhutan', '{"type":"Polygon","coordinates":[[...]]}')
        ) AS t(name, geojson)
        ON CONFLICT (name, kind) DO UPDATE SET geometry = excluded.geometry
        "#,
    )
    .execute(pool)
    .await
    .context("syncing country bounds")?;

    Ok(())
}

pub async fn sync_regions(pool: &PgPool) -> Result<()> {
    tracing::info!("syncing regions from OpenStreetMap");

    sqlx::query(
        "INSERT INTO regions (name, parent_name, kind, geometry, osm_id)
         SELECT tags->>'name', tags->>'name:parent', 'region', geometry, osm_id
         FROM raw_areas
         WHERE (tags->>'admin_level')::int <= 6
         ON CONFLICT (name, kind) DO UPDATE SET geometry = excluded.geometry",
    )
    .execute(pool)
    .await
    .context("syncing regions")?;

    Ok(())
}

pub async fn sync_places(pool: &PgPool) -> Result<()> {
    tracing::info!("syncing places from OpenStreetMap");

    sqlx::query(
        "INSERT INTO places (name, kind, geometry, osm_id)
         SELECT tags->>'name', tags->>'place', geometry, osm_id
         FROM raw_points
         WHERE tags->>'place' IN ('town', 'village', 'hamlet', 'city')
         ON CONFLICT (name, kind) DO UPDATE SET geometry = excluded.geometry",
    )
    .execute(pool)
    .await
    .context("syncing places")?;

    Ok(())
}

pub async fn sync_outdoor_waypoints(pool: &PgPool) -> Result<()> {
    tracing::info!("syncing outdoor waypoints");

    sqlx::query(
        "INSERT INTO outdoor_waypoints (name, kind, geometry, osm_id)
         SELECT tags->>'name', tags->>'tourism', geometry, osm_id
         FROM raw_points
         WHERE tags->>'tourism' IN ('alpine_hut', 'camp_site', 'guest_house')
         ON CONFLICT (osm_id) DO UPDATE SET geometry = excluded.geometry",
    )
    .execute(pool)
    .await
    .context("syncing outdoor waypoints")?;

    Ok(())
}
