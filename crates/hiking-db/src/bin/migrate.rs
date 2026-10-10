use anyhow::{Context, Result};
use sqlx::migrate::Migrator;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "hiking_migrate=info,sqlx=warn".into()),
        )
        .init();

    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
    let migrations_dir = std::env::var("MIGRATIONS_DIR")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "migrations".to_string());

    let pool = hiking_db::create_pool(&database_url).await?;
    let migrator = Migrator::new(Path::new(&migrations_dir))
        .await
        .with_context(|| format!("loading migrations from {migrations_dir}"))?;
    migrator
        .run(&pool)
        .await
        .context("applying database migrations")?;

    tracing::info!(migrations_dir, "database migrations are up to date");
    Ok(())
}
