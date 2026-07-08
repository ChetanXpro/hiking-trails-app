pub mod models;
pub use models::decode_wkb;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// Create a Postgres connection pool.
/// Expects a standard connection string, e.g.:
///   postgres://user:password@localhost/hiking
pub async fn create_pool(database_url: &str) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;
    Ok(pool)
}
