pub mod models;
pub use models::decode_wkb;

use anyhow::{bail, Context};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::PgPool;
use std::str::FromStr;
use std::time::Duration;

const DEFAULT_POOL_MAX_CONNECTIONS: u32 = 10;
const DEFAULT_POOL_MIN_CONNECTIONS: u32 = 1;
const DEFAULT_CONNECT_TIMEOUT_SECONDS: u64 = 10;
const DEFAULT_ACQUIRE_TIMEOUT_SECONDS: u64 = 5;
const DEFAULT_IDLE_TIMEOUT_SECONDS: u64 = 600;
const DEFAULT_MAX_LIFETIME_SECONDS: u64 = 1_800;
const DEFAULT_STATEMENT_TIMEOUT_MS: u64 = 15_000;
const DEFAULT_LOCK_TIMEOUT_MS: u64 = 3_000;

fn env_u32(key: &str, default: u32) -> anyhow::Result<u32> {
    match std::env::var(key) {
        Ok(value) if !value.trim().is_empty() => value
            .trim()
            .parse::<u32>()
            .with_context(|| format!("parsing {key} as an unsigned integer")),
        _ => Ok(default),
    }
}

fn env_u64(key: &str, default: u64) -> anyhow::Result<u64> {
    match std::env::var(key) {
        Ok(value) if !value.trim().is_empty() => value
            .trim()
            .parse::<u64>()
            .with_context(|| format!("parsing {key} as an unsigned integer")),
        _ => Ok(default),
    }
}

/// Create a Postgres connection pool.
/// Expects a standard connection string, e.g.:
///   postgres://user:password@localhost/hiking
pub async fn create_pool(database_url: &str) -> anyhow::Result<PgPool> {
    let max_connections = env_u32("DB_POOL_MAX_CONNECTIONS", DEFAULT_POOL_MAX_CONNECTIONS)?;
    let min_connections = env_u32("DB_POOL_MIN_CONNECTIONS", DEFAULT_POOL_MIN_CONNECTIONS)?;
    if max_connections == 0 {
        bail!("DB_POOL_MAX_CONNECTIONS must be at least 1");
    }
    if min_connections > max_connections {
        bail!("DB_POOL_MIN_CONNECTIONS cannot exceed DB_POOL_MAX_CONNECTIONS");
    }

    let connect_timeout = Duration::from_secs(env_u64(
        "DB_CONNECT_TIMEOUT_SECONDS",
        DEFAULT_CONNECT_TIMEOUT_SECONDS,
    )?);
    let acquire_timeout = Duration::from_secs(env_u64(
        "DB_ACQUIRE_TIMEOUT_SECONDS",
        DEFAULT_ACQUIRE_TIMEOUT_SECONDS,
    )?);
    let idle_timeout = Duration::from_secs(env_u64(
        "DB_IDLE_TIMEOUT_SECONDS",
        DEFAULT_IDLE_TIMEOUT_SECONDS,
    )?);
    let max_lifetime = Duration::from_secs(env_u64(
        "DB_MAX_LIFETIME_SECONDS",
        DEFAULT_MAX_LIFETIME_SECONDS,
    )?);
    let statement_timeout_ms = env_u64("DB_STATEMENT_TIMEOUT_MS", DEFAULT_STATEMENT_TIMEOUT_MS)?;
    let lock_timeout_ms = env_u64("DB_LOCK_TIMEOUT_MS", DEFAULT_LOCK_TIMEOUT_MS)?;
    let application_name = std::env::var("DB_APPLICATION_NAME")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "hiking-trails-app".to_string());

    let options = PgConnectOptions::from_str(database_url)
        .context("parsing DATABASE_URL")?
        .application_name(&application_name);
    let pool_options = PgPoolOptions::new()
        .max_connections(max_connections)
        .min_connections(min_connections)
        .acquire_timeout(acquire_timeout)
        .idle_timeout(Some(idle_timeout))
        .max_lifetime(Some(max_lifetime))
        .test_before_acquire(true)
        .after_connect(move |connection, _metadata| {
            Box::pin(async move {
                sqlx::query(
                    r#"
                    SELECT
                        set_config('statement_timeout', $1, false),
                        set_config('lock_timeout', $2, false)
                    "#,
                )
                .bind(format!("{statement_timeout_ms}ms"))
                .bind(format!("{lock_timeout_ms}ms"))
                .execute(connection)
                .await?;
                Ok(())
            })
        });

    let pool = tokio::time::timeout(connect_timeout, pool_options.connect_with(options))
        .await
        .context("timed out connecting to Postgres")??;
    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postgres_url_is_validated_before_connecting() {
        assert!(PgConnectOptions::from_str("not a postgres URL").is_err());
    }
}
