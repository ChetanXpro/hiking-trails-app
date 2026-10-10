use anyhow::{Context, Result};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

pub mod environment;
pub use environment::*;

/// GDAL version and feature support
#[derive(Debug, Clone, Copy)]
pub struct GdalCapabilities {
    pub supports_batch_queries: bool,
    pub supports_interpolation_flag: bool,
}

/// Detect GDAL version and capabilities
pub fn detect_gdal_capabilities() -> Result<GdalCapabilities> {
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
        .and_then(|s| s.parse::<u32>().ok())
        .context("missing or invalid major version")?;
    let minor = parts
        .next()
        .and_then(|s| s.parse::<u32>().ok())
        .context("missing or invalid minor version")?;

    Ok((major, minor))
}

/// Helper to parse environment variables
pub fn env_f64(key: &str, default: f64) -> Result<f64> {
    match std::env::var(key) {
        Ok(value) if !value.trim().is_empty() => value
            .trim()
            .parse::<f64>()
            .with_context(|| format!("parsing {} as f64", key)),
        _ => Ok(default),
    }
}

/// Helper to parse environment variables
pub fn env_usize(key: &str, default: usize) -> Result<usize> {
    match std::env::var(key) {
        Ok(value) if !value.trim().is_empty() => value
            .trim()
            .parse::<usize>()
            .with_context(|| format!("parsing {} as usize", key)),
        _ => Ok(default),
    }
}

/// Helper to parse environment variables
pub fn env_bool(key: &str, default: bool) -> Result<bool> {
    let value = std::env::var(key).ok();
    parse_bool_value(key, value.as_deref(), default)
}

fn parse_bool_value(key: &str, value: Option<&str>, default: bool) -> Result<bool> {
    match value {
        Some(value) if !value.trim().is_empty() => {
            match value.trim().to_ascii_lowercase().as_str() {
                "1" | "true" | "yes" | "on" => Ok(true),
                "0" | "false" | "no" | "off" => Ok(false),
                _ => anyhow::bail!("{key} must be true or false"),
            }
        }
        _ => Ok(default),
    }
}

/// Find TIF files in a directory
pub fn find_tif_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let entries = std::fs::read_dir(dir)
        .with_context(|| format!("reading directory {}", dir.display()))?;

    let paths = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    matches!(ext_str.as_str(), "tif" | "tiff")
                })
        })
        .map(|e| e.path())
        .collect::<Vec<_>>();

    Ok(paths)
}

/// Get path from environment variable
pub fn non_blank_path(var: Option<OsString>) -> Option<PathBuf> {
    var.and_then(|path| {
        let path = PathBuf::from(path);
        if path.as_os_str().is_empty() {
            None
        } else {
            Some(path)
        }
    })
}
