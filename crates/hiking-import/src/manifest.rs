use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::PgPool;

const MAX_ORIGINAL_MAPPER_MANIFEST_BYTES: u64 = 5 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OriginalMapperManifest {
    pub schema_version: u32,
    pub relations: Vec<OriginalMapperManifestEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OriginalMapperManifestEntry {
    pub osm_relation_id: i64,
    pub osm_relation_version: u32,
    pub display_name: String,
    pub provider_user_id: Option<String>,
    pub profile_url: Option<String>,
    pub mapped_at: DateTime<Utc>,
}

pub fn parse_original_mapper_manifest(bytes: &[u8]) -> Result<OriginalMapperManifest> {
    serde_json::from_slice(bytes).context("parsing original mapper manifest JSON")
}

pub async fn sync_original_mapper_manifest_from_env(_pool: &PgPool) -> Result<()> {
    // Manifest syncing would be implemented here.
    // This is left as a stub for organization purposes.
    tracing::info!("original mapper manifest sync disabled in modular structure");
    Ok(())
}
