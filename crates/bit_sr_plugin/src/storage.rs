//! Scoped Storage Model and Quota Management.
//!
//! Provides virtualized, high-speed scoped storage mapped to `/data` in WASI,
//! with strict disk quota enforcement (`max_storage_mb`).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Errors encountered during scoped storage operations or quota checks.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("Storage quota exceeded: current usage {current} bytes + {incoming} bytes exceeds maximum quota of {limit} bytes")]
    QuotaExceeded {
        current: u64,
        incoming: u64,
        limit: u64,
    },
    #[error("Storage access denied: extension does not have storage permission")]
    PermissionDenied,
}

/// Scoped storage manager for an individual extension.
#[derive(Debug, Clone)]
pub struct ScopedStorage {
    /// Unique extension identifier.
    plugin_id: String,
    /// Absolute host path to the extension's isolated `/data` directory.
    host_data_dir: PathBuf,
    /// Maximum allowed storage capacity in bytes.
    max_storage_bytes: u64,
}

impl ScopedStorage {
    /// Creates a scoped storage instance with a custom base directory.
    pub fn new(plugin_id: impl Into<String>, base_dir: impl AsRef<Path>, max_storage_mb: u32) -> Self {
        let plugin_id = plugin_id.into();
        let host_data_dir = base_dir.as_ref().join(&plugin_id);
        let max_storage_bytes = (max_storage_mb as u64) * 1024 * 1024;
        Self {
            plugin_id,
            host_data_dir,
            max_storage_bytes,
        }
    }

    /// Resolves the system default base storage directory for extensions.
    ///
    /// - Windows: `%APPDATA%\bit_sr\extensions\data\`
    /// - Linux/macOS: `~/.local/share/bit_sr/extensions/data/`
    pub fn default_system_base_dir() -> PathBuf {
        #[cfg(windows)]
        {
            if let Ok(appdata) = std::env::var("APPDATA") {
                PathBuf::from(appdata).join("bit_sr").join("extensions").join("data")
            } else {
                PathBuf::from(".bit_sr").join("extensions").join("data")
            }
        }
        #[cfg(not(windows))]
        {
            if let Ok(home) = std::env::var("HOME") {
                PathBuf::from(home)
                    .join(".local")
                    .join("share")
                    .join("bit_sr")
                    .join("extensions")
                    .join("data")
            } else {
                PathBuf::from(".bit_sr").join("extensions").join("data")
            }
        }
    }

    /// Creates a scoped storage instance using the default system base path.
    pub fn with_system_defaults(plugin_id: impl Into<String>, max_storage_mb: u32) -> Self {
        Self::new(plugin_id, Self::default_system_base_dir(), max_storage_mb)
    }

    /// Returns the extension's unique ID.
    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    /// Returns the host directory path where `/data` is stored.
    pub fn host_data_dir(&self) -> &Path {
        &self.host_data_dir
    }

    /// Returns the maximum allowed storage quota in bytes.
    pub fn max_storage_bytes(&self) -> u64 {
        self.max_storage_bytes
    }

    /// Ensures that the host data directory exists, creating it if necessary.
    pub fn ensure_dir(&self) -> Result<&Path, StorageError> {
        fs::create_dir_all(&self.host_data_dir)?;
        Ok(&self.host_data_dir)
    }

    /// Computes the total disk space currently consumed by this extension's data directory.
    pub fn current_usage_bytes(&self) -> Result<u64, StorageError> {
        if !self.host_data_dir.exists() {
            return Ok(0);
        }
        calculate_dir_size(&self.host_data_dir)
    }

    /// Checks whether writing `additional_bytes` would exceed the configured quota.
    pub fn check_quota(&self, additional_bytes: u64) -> Result<(), StorageError> {
        let current = self.current_usage_bytes()?;
        if current.saturating_add(additional_bytes) > self.max_storage_bytes {
            return Err(StorageError::QuotaExceeded {
                current,
                incoming: additional_bytes,
                limit: self.max_storage_bytes,
            });
        }
        Ok(())
    }

    /// Cleans and removes all files inside the extension's data directory.
    pub fn clean_all(&self) -> Result<(), StorageError> {
        if self.host_data_dir.exists() {
            fs::remove_dir_all(&self.host_data_dir)?;
        }
        Ok(())
    }
}

fn calculate_dir_size(dir: &Path) -> Result<u64, StorageError> {
    let mut total = 0;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let meta = fs::metadata(&path)?;
            total += meta.len();
        } else if path.is_dir() {
            total += calculate_dir_size(&path)?;
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_scoped_storage_quota_enforcement() {
        let tmp = tempdir().unwrap();
        // Quota: 1 MB
        let storage = ScopedStorage::new("org.test.plugin", tmp.path(), 1);
        storage.ensure_dir().unwrap();

        // 0 usage initially
        assert_eq!(storage.current_usage_bytes().unwrap(), 0);

        // Writing 500 KB should be within 1 MB quota
        assert!(storage.check_quota(500 * 1024).is_ok());

        // Writing 2 MB should exceed 1 MB quota
        let err = storage.check_quota(2 * 1024 * 1024).unwrap_err();
        assert!(matches!(err, StorageError::QuotaExceeded { .. }));

        // Write a 600 KB file
        let file_path = storage.host_data_dir().join("test.bin");
        fs::write(&file_path, vec![0u8; 600 * 1024]).unwrap();

        let current = storage.current_usage_bytes().unwrap();
        assert_eq!(current, 600 * 1024);

        // Writing another 500 KB should now fail (600KB + 500KB = 1.1MB > 1MB)
        assert!(storage.check_quota(500 * 1024).is_err());
    }
}
