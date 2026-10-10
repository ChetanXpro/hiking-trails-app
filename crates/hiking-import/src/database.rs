use anyhow::Result;
use serde_json::Value;
use sqlx::{types::Json, Postgres, QueryBuilder, PgConnection};

const INSERT_BATCH_ROWS: usize = 1_000;

pub async fn insert_segments_batch(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    trail_id: i32,
    segments: Vec<(String, Option<i32>, Option<String>)>,
) -> Result<()> {
    if segments.is_empty() {
        return Ok(());
    }
    let mut qb = QueryBuilder::new(
        "INSERT INTO trail_segments (trail_id, geometry, surface_id, surface_type)",
    );
    qb.push_values(segments.iter().take(INSERT_BATCH_ROWS), |mut b, (geom, surface_id, surface_type)| {
        b.push_bind(trail_id)
            .push_bind(geom)
            .push_bind(surface_id)
            .push_bind(surface_type);
    });
    qb.push(" ON CONFLICT DO NOTHING");
    qb.build().execute(&mut **tx).await?;
    Ok(())
}

pub async fn insert_waypoints_batch(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    trail_id: i32,
    waypoints: Vec<(String, String, Option<f64>)>,
) -> Result<()> {
    if waypoints.is_empty() {
        return Ok(());
    }
    let mut qb = QueryBuilder::new(
        "INSERT INTO trail_waypoints (trail_id, geometry, name, elevation_m)",
    );
    qb.push_values(waypoints.iter().take(INSERT_BATCH_ROWS), |mut b, (geom, name, elev)| {
        b.push_bind(trail_id)
            .push_bind(geom)
            .push_bind(name)
            .push_bind(elev);
    });
    qb.push(" ON CONFLICT DO NOTHING");
    qb.build().execute(&mut **tx).await?;
    Ok(())
}

pub async fn insert_surface_breakdown_batch(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    trail_id: i32,
    breakdown: &Value,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO trail_surface_breakdown (trail_id, breakdown) VALUES ($1, $2) ON CONFLICT (trail_id) DO UPDATE SET breakdown = excluded.breakdown",
    )
    .bind(trail_id)
    .bind(Json(breakdown))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn insert_trail_features_batch(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    trail_id: i32,
    features: Vec<(String, String)>,
) -> Result<()> {
    if features.is_empty() {
        return Ok(());
    }
    let mut qb = QueryBuilder::new(
        "INSERT INTO trail_features (trail_id, feature_type, geometry)",
    );
    qb.push_values(features.iter().take(INSERT_BATCH_ROWS), |mut b, (kind, geom)| {
        b.push_bind(trail_id)
            .push_bind(kind)
            .push_bind(geom);
    });
    qb.push(" ON CONFLICT DO NOTHING");
    qb.build().execute(&mut **tx).await?;
    Ok(())
}

pub async fn insert_landcover_segments_batch(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    trail_id: i32,
    segments: Vec<(f64, f64, String)>,
) -> Result<()> {
    if segments.is_empty() {
        return Ok(());
    }
    let mut qb = QueryBuilder::new(
        "INSERT INTO trail_landcover_breakdown (trail_id, distance_m, portion, landcover_class)",
    );
    qb.push_values(segments.iter().take(INSERT_BATCH_ROWS), |mut b, (dist, portion, class)| {
        b.push_bind(trail_id)
            .push_bind(dist)
            .push_bind(portion)
            .push_bind(class);
    });
    qb.push(" ON CONFLICT DO NOTHING");
    qb.build().execute(&mut **tx).await?;
    Ok(())
}

pub async fn insert_stitch_warnings_batch(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    trail_id: i32,
    warnings: Vec<String>,
) -> Result<()> {
    if warnings.is_empty() {
        return Ok(());
    }
    let mut qb = QueryBuilder::new(
        "INSERT INTO trail_stitch_warnings (trail_id, warning)",
    );
    qb.push_values(warnings.iter().take(INSERT_BATCH_ROWS), |mut b, warning| {
        b.push_bind(trail_id)
            .push_bind(warning);
    });
    qb.push(" ON CONFLICT DO NOTHING");
    qb.build().execute(&mut **tx).await?;
    Ok(())
}

pub async fn insert_diagnostic_points_batch(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    trail_id: i32,
    points: Vec<(String, String)>,
) -> Result<()> {
    if points.is_empty() {
        return Ok(());
    }
    let mut qb = QueryBuilder::new(
        "INSERT INTO trail_diagnostic_points (trail_id, geometry, point_type)",
    );
    qb.push_values(points.iter().take(INSERT_BATCH_ROWS), |mut b, (geom, ptype)| {
        b.push_bind(trail_id)
            .push_bind(geom)
            .push_bind(ptype);
    });
    qb.push(" ON CONFLICT DO NOTHING");
    qb.build().execute(&mut **tx).await?;
    Ok(())
}
