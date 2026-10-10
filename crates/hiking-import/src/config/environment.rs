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
        let prune_missing_trails = super::env_bool("IMPORT_PRUNE_MISSING_TRAILS", false)?;

        Ok(Self {
            database_url,
            prune_missing_trails,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_requires_database_url() {
        std::env::remove_var("DATABASE_URL");
        assert!(ImportConfig::from_env().is_err());
    }

    #[test]
    fn test_config_defaults_prune_missing_trails() {
        std::env::set_var("DATABASE_URL", "postgres://localhost");
        std::env::remove_var("IMPORT_PRUNE_MISSING_TRAILS");
        let config = ImportConfig::from_env().expect("should load config");
        assert!(!config.prune_missing_trails);
    }
}
