use anyhow::{Context, Result};

/// Import service configuration from environment variables
#[derive(Debug, Clone)]
pub struct ImportConfig {
    /// Database connection URL
    pub database_url: String,
    /// Prune missing trails from database during import
    pub prune_missing_trails: bool,
}

impl ImportConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
        let prune_missing_trails = std::env::var("IMPORT_PRUNE_MISSING_TRAILS").ok();

        Self::from_values(Some(&database_url), prune_missing_trails.as_deref())
    }

    fn from_values(database_url: Option<&str>, prune_missing_trails: Option<&str>) -> Result<Self> {
        let database_url = database_url.context("DATABASE_URL must be set")?;
        let prune_missing_trails =
            super::parse_bool_value("IMPORT_PRUNE_MISSING_TRAILS", prune_missing_trails, false)?;

        Ok(Self {
            database_url: database_url.to_string(),
            prune_missing_trails,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_requires_database_url() {
        assert!(ImportConfig::from_values(None, None).is_err());
    }

    #[test]
    fn test_config_defaults_prune_missing_trails() {
        let config = ImportConfig::from_values(Some("postgres://localhost"), None)
            .expect("should load config");
        assert!(!config.prune_missing_trails);
    }
}
