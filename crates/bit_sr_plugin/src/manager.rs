//! Extension Manager & Lifecycle Orchestrator.
//!
//! Handles installation, uninstallation, enabling/disabling, permission updates,
//! and metadata queries for `.bsp` extension packages.

use crate::host_bridge::{PluginContext, PluginSpeechSink};
use crate::manifest::ExtensionTier;
use crate::package::{PackageError, PluginPackage};
use crate::permissions::{GrantedPermissions, PermissionType};
use crate::storage::{ScopedStorage, StorageError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Operational lifecycle state of an extension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExtensionStatus {
    /// Active and responding to events.
    Active,
    /// Paused (e.g. exceeded execution epoch deadline).
    Paused { reason: String },
    /// Disabled by the user.
    Disabled,
    /// Failed to load or encountered fatal error.
    Error { message: String },
}

/// User-facing summary info for an extension (e.g. for the Slint UI).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub tier: ExtensionTier,
    pub status: ExtensionStatus,
    pub enabled: bool,
    pub permissions: GrantedPermissions,
    pub storage_usage_bytes: u64,
    pub max_storage_bytes: u64,
    pub max_memory_mb: u32,
    pub hotkeys: Vec<String>,
}

/// Internal record of an installed extension.
pub struct PluginRecord {
    pub package: PluginPackage,
    pub status: ExtensionStatus,
    pub enabled: bool,
    pub permissions: Arc<RwLock<GrantedPermissions>>,
    pub storage: ScopedStorage,
}

/// Errors originating from extension management operations.
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Package error: {0}")]
    Package(#[from] PackageError),
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("Extension '{0}' is not installed")]
    NotFound(String),
    #[error("Extension '{0}' is already installed")]
    AlreadyInstalled(String),
    #[error("Failed to load or execute extension: {0}")]
    Execution(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Top-level coordinator managing installed extensions and their lifecycles.
pub struct PluginManager {
    /// Directory where installed extensions are unpacked.
    extensions_dir: PathBuf,
    /// Directory where virtualized `/data` scoped storages reside.
    data_dir: PathBuf,
    /// Map of installed plugin records keyed by plugin ID.
    plugins: RwLock<HashMap<String, PluginRecord>>,
    /// Optional speech sink for extension speech output.
    speech_sink: Option<Arc<dyn PluginSpeechSink>>,
}

impl PluginManager {
    /// Creates a new plugin manager with explicit directory paths.
    pub fn new(
        extensions_dir: impl AsRef<Path>,
        data_dir: impl AsRef<Path>,
        speech_sink: Option<Arc<dyn PluginSpeechSink>>,
    ) -> Result<Self, PluginError> {
        let extensions_dir = extensions_dir.as_ref().to_path_buf();
        let data_dir = data_dir.as_ref().to_path_buf();

        fs::create_dir_all(&extensions_dir)?;
        fs::create_dir_all(&data_dir)?;

        Ok(Self {
            extensions_dir,
            data_dir,
            plugins: RwLock::new(HashMap::new()),
            speech_sink,
        })
    }

    /// Creates a plugin manager using system default paths.
    pub fn with_system_defaults(
        speech_sink: Option<Arc<dyn PluginSpeechSink>>,
    ) -> Result<Self, PluginError> {
        let base = ScopedStorage::default_system_base_dir();
        let extensions_dir = base.parent().unwrap_or(&base).join("installed");
        Self::new(extensions_dir, base, speech_sink)
    }

    /// Installs an unpacked extension package.
    pub fn install_package(
        &self,
        package: PluginPackage,
        grant_system_access: bool,
    ) -> Result<String, PluginError> {
        let id = package.manifest.plugin.id.clone();
        let mut plugins = self.plugins.write().unwrap();

        if plugins.contains_key(&id) {
            return Err(PluginError::AlreadyInstalled(id));
        }

        // Unpack into installed directory
        let install_path = self.extensions_dir.join(&id);
        package.extract_to_dir(&install_path)?;

        // Initialize scoped storage and granted permissions
        let storage = ScopedStorage::new(&id, &self.data_dir, package.manifest.resources.max_storage_mb);
        storage.ensure_dir()?;

        let perms = GrantedPermissions::from_manifest(&package.manifest, grant_system_access);
        let perms_lock = Arc::new(RwLock::new(perms));

        let record = PluginRecord {
            package,
            status: ExtensionStatus::Active,
            enabled: true,
            permissions: perms_lock,
            storage,
        };

        plugins.insert(id.clone(), record);
        log::info!("Installed extension: {}", id);
        Ok(id)
    }

    /// Installs an extension from a `.bsp` file archive.
    pub fn install_bsp_file(
        &self,
        bsp_path: impl AsRef<Path>,
        grant_system_access: bool,
    ) -> Result<String, PluginError> {
        let package = PluginPackage::from_file(bsp_path)?;
        self.install_package(package, grant_system_access)
    }

    /// Installs an extension from `.bsp` byte buffer.
    pub fn install_bsp_bytes(
        &self,
        bytes: &[u8],
        grant_system_access: bool,
    ) -> Result<String, PluginError> {
        let package = PluginPackage::from_bytes(bytes)?;
        self.install_package(package, grant_system_access)
    }

    /// Uninstalls an extension, purging its files and storage.
    pub fn uninstall(&self, plugin_id: &str) -> Result<(), PluginError> {
        let mut plugins = self.plugins.write().unwrap();
        let record = plugins
            .remove(plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;

        // Remove installed extension directory
        let install_path = self.extensions_dir.join(plugin_id);
        if install_path.exists() {
            let _ = fs::remove_dir_all(install_path);
        }

        // Clean `/data` storage
        let _ = record.storage.clean_all();

        log::info!("Uninstalled extension: {}", plugin_id);
        Ok(())
    }

    /// Enables an extension.
    pub fn enable(&self, plugin_id: &str) -> Result<(), PluginError> {
        let mut plugins = self.plugins.write().unwrap();
        let record = plugins
            .get_mut(plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;

        record.enabled = true;
        record.status = ExtensionStatus::Active;
        log::info!("Enabled extension: {}", plugin_id);
        Ok(())
    }

    /// Disables an extension.
    pub fn disable(&self, plugin_id: &str) -> Result<(), PluginError> {
        let mut plugins = self.plugins.write().unwrap();
        let record = plugins
            .get_mut(plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;

        record.enabled = false;
        record.status = ExtensionStatus::Disabled;
        log::info!("Disabled extension: {}", plugin_id);
        Ok(())
    }

    /// Updates or toggles a specific permission for an installed extension at runtime.
    pub fn set_permission(
        &self,
        plugin_id: &str,
        permission: PermissionType,
        granted: bool,
    ) -> Result<(), PluginError> {
        let plugins = self.plugins.read().unwrap();
        let record = plugins
            .get(plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;

        let mut perms = record.permissions.write().unwrap();
        if granted {
            perms.grant(permission);
        } else {
            perms.revoke(permission);
        }
        log::info!(
            "Extension '{}' permission '{:?}' set to {}",
            plugin_id,
            permission,
            granted
        );
        Ok(())
    }

    /// Queries detailed user-facing information for an extension.
    pub fn get_plugin_info(&self, plugin_id: &str) -> Option<PluginInfo> {
        let plugins = self.plugins.read().unwrap();
        let record = plugins.get(plugin_id)?;
        let manifest = &record.package.manifest;
        let perms = record.permissions.read().unwrap().clone();
        let usage = record.storage.current_usage_bytes().unwrap_or(0);

        Some(PluginInfo {
            id: manifest.plugin.id.clone(),
            name: manifest.plugin.name.clone(),
            version: manifest.plugin.version.clone(),
            author: manifest.plugin.author.clone(),
            description: manifest.plugin.description.clone(),
            tier: manifest.tier(),
            status: record.status.clone(),
            enabled: record.enabled,
            permissions: perms,
            storage_usage_bytes: usage,
            max_storage_bytes: record.storage.max_storage_bytes(),
            max_memory_mb: manifest.resources.max_memory_mb,
            hotkeys: manifest.permissions.hotkeys.clone(),
        })
    }

    /// Lists summary info for all installed extensions.
    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        let plugins = self.plugins.read().unwrap();
        let mut list = Vec::new();
        for id in plugins.keys() {
            if let Some(info) = self.get_plugin_info(id) {
                list.push(info);
            }
        }
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }

    /// Builds a `PluginContext` for an installed extension.
    pub fn create_context(&self, plugin_id: &str) -> Option<PluginContext> {
        let plugins = self.plugins.read().unwrap();
        let record = plugins.get(plugin_id)?;
        Some(PluginContext::new(
            record.package.manifest.clone(),
            record.permissions.clone(),
            self.data_dir.clone(),
            self.speech_sink.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PluginManifest;
    use tempfile::tempdir;

    #[test]
    fn test_plugin_manager_lifecycle() {
        let tmp = tempdir().unwrap();
        let extensions_dir = tmp.path().join("installed");
        let data_dir = tmp.path().join("data");

        let manager = PluginManager::new(&extensions_dir, &data_dir, None).unwrap();

        let toml_str = r#"
[plugin]
id = "org.bitsr.notes"
name = "Quick Notes"
version = "1.0.0"
author = "Dev Team"
description = "Take fast notes"

[resources]
max_memory_mb = 64
max_storage_mb = 10

[permissions]
storage = true
speech = { output = true, filter = false }
"#;
        let manifest = PluginManifest::from_toml_str(toml_str).unwrap();
        let dummy_wasm = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
        let package = PluginPackage::new(manifest, dummy_wasm, None);

        // Install
        let id = manager.install_package(package, false).unwrap();
        assert_eq!(id, "org.bitsr.notes");

        // Verify installed info
        let info = manager.get_plugin_info(&id).unwrap();
        assert_eq!(info.name, "Quick Notes");
        assert_eq!(info.tier, ExtensionTier::Tier1Standard);
        assert!(info.enabled);
        assert_eq!(info.status, ExtensionStatus::Active);
        assert!(info.permissions.storage);

        // Disable
        manager.disable(&id).unwrap();
        let info_disabled = manager.get_plugin_info(&id).unwrap();
        assert!(!info_disabled.enabled);
        assert_eq!(info_disabled.status, ExtensionStatus::Disabled);

        // Toggle permission at runtime
        manager
            .set_permission(&id, PermissionType::SpeechOutput, false)
            .unwrap();
        let info_perm = manager.get_plugin_info(&id).unwrap();
        assert!(!info_perm.permissions.speech_output);

        // Uninstall
        manager.uninstall(&id).unwrap();
        assert!(manager.get_plugin_info(&id).is_none());
        assert_eq!(manager.list_plugins().len(), 0);
    }
}
