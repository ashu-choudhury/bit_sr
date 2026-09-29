//! Plugin Manifest parsing, validation, and metadata models.
//!
//! Parses `manifest.toml` adhering to the `bit_sr` WebAssembly Extension Ecosystem Specification.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Extension security tier based on declared capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExtensionTier {
    /// Tier 1: Standard sandboxed extension.
    ///
    /// - Scoped storage only (`/data`)
    /// - Strict memory quota
    /// - Granular capabilities (speech, network, hotkeys)
    /// - Zero host OS / DLL / process execution access
    Tier1Standard,

    /// Tier 2: System access extension.
    ///
    /// - Full unrestricted host bridge access (any DLL, system commands, raw hardware)
    /// - Requires explicit user trust and consent confirmation dialog
    Tier2SystemAccess,
}

impl ExtensionTier {
    /// Returns the user-facing display label for the tier.
    pub fn display_label(&self) -> &'static str {
        match self {
            Self::Tier1Standard => "Tier 1: Sandboxed Extension 🛡️",
            Self::Tier2SystemAccess => "Tier 2: System Access Extension 🔑",
        }
    }

    /// Whether this tier requires high-risk system access approval.
    pub fn is_system_access(&self) -> bool {
        matches!(self, Self::Tier2SystemAccess)
    }
}

/// Metadata identifying the extension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Unique reverse-DNS identifier (e.g., "org.bitsr.quick_notes").
    pub id: String,
    /// User-friendly name of the extension.
    pub name: String,
    /// Semantic version string (e.g., "1.2.0").
    pub version: String,
    /// Author or organization name.
    pub author: String,
    /// Brief description of what the extension provides.
    pub description: String,
    /// Minimum compatible `bit_sr` version.
    #[serde(default)]
    pub min_bit_sr_version: Option<String>,
    /// Optional homepage or repository URL.
    #[serde(default)]
    pub homepage: Option<String>,
    /// Optional license string (defaults to MIT OR Apache-2.0).
    #[serde(default = "default_license")]
    pub license: String,
}

fn default_license() -> String {
    "MIT OR Apache-2.0".to_string()
}

/// Resource quotas enforced by Wasmtime and the host environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceConfig {
    /// Maximum RAM allocation in megabytes (default 64 MB).
    #[serde(default = "default_max_memory_mb")]
    pub max_memory_mb: u32,
    /// Maximum disk storage allocation in megabytes inside `/data` (default 25 MB).
    #[serde(default = "default_max_storage_mb")]
    pub max_storage_mb: u32,
    /// Optional execution deadline per event callback in milliseconds (default 500 ms).
    #[serde(default = "default_execution_timeout_ms")]
    pub execution_timeout_ms: u64,
}

impl Default for ResourceConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: default_max_memory_mb(),
            max_storage_mb: default_max_storage_mb(),
            execution_timeout_ms: default_execution_timeout_ms(),
        }
    }
}

fn default_max_memory_mb() -> u32 {
    64
}

fn default_max_storage_mb() -> u32 {
    25
}

fn default_execution_timeout_ms() -> u64 {
    500
}

/// Speech capability permissions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SpeechPermissionConfig {
    /// Permission to speak strings or play audio cues.
    #[serde(default)]
    pub output: bool,
    /// Permission to inspect and filter outgoing speech text.
    #[serde(default)]
    pub filter: bool,
}

/// Network capability permissions.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct NetworkPermissionConfig {
    /// Permission to make outbound network requests.
    #[serde(default)]
    pub access: bool,
    /// Explicit whitelist of allowed domain names.
    #[serde(default)]
    pub allowed_domains: Vec<String>,
}

/// Accessibility tree inspection and interaction permissions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AccessibilityPermissionConfig {
    /// Permission to read the currently focused node (role, name, states).
    #[serde(default)]
    pub read: bool,
    /// Permission to walk the accessibility tree (parents, children, siblings).
    #[serde(default)]
    pub walk: bool,
    /// Permission to invoke accessible actions (click, toggle, etc.).
    #[serde(default)]
    pub interact: bool,
}

/// Declared capabilities and permissions in `manifest.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PermissionConfig {
    /// Access to virtualized scoped storage at `/data`.
    #[serde(default)]
    pub storage: bool,
    /// Speech output and speech filter permissions.
    #[serde(default)]
    pub speech: SpeechPermissionConfig,
    /// Network fetch capability and domain whitelisting.
    #[serde(default)]
    pub network: NetworkPermissionConfig,
    /// Registered keyboard shortcut bindings (e.g. `["SR + Shift + N"]`).
    #[serde(default)]
    pub hotkeys: Vec<String>,
    /// System Access Capability (Tier 2 - Unlocks full OS control).
    #[serde(default)]
    pub system_access: bool,
    /// Accessibility tree inspection capabilities.
    #[serde(default)]
    pub accessibility: AccessibilityPermissionConfig,
}

/// Complete manifest definition for a `bit_sr` extension package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Extension metadata.
    pub plugin: PluginMetadata,
    /// Resource quotas and timeout configurations.
    #[serde(default)]
    pub resources: ResourceConfig,
    /// Declared capability permissions.
    #[serde(default)]
    pub permissions: PermissionConfig,
}

/// Errors that can occur during manifest parsing or validation.
#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("Failed to parse TOML manifest: {0}")]
    ParseError(#[from] toml::de::Error),
    #[error("Failed to serialize TOML manifest: {0}")]
    SerializeError(#[from] toml::ser::Error),
    #[error("Manifest validation error: {0}")]
    ValidationError(String),
}

impl PluginManifest {
    /// Parse a manifest from a TOML string.
    pub fn from_toml_str(toml_str: &str) -> Result<Self, ManifestError> {
        let manifest: Self = toml::from_str(toml_str)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Serialize the manifest to a TOML string.
    pub fn to_toml_string(&self) -> Result<String, ManifestError> {
        Ok(toml::to_string_pretty(self)?)
    }

    /// Returns the security tier of the extension based on requested capabilities.
    pub fn tier(&self) -> ExtensionTier {
        if self.permissions.system_access {
            ExtensionTier::Tier2SystemAccess
        } else {
            ExtensionTier::Tier1Standard
        }
    }

    /// Validates the fields of the manifest.
    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.plugin.id.trim().is_empty() {
            return Err(ManifestError::ValidationError(
                "Plugin ID cannot be empty".to_string(),
            ));
        }

        // Validate plugin ID characters: alphanumeric, dot, underscore, dash
        if !self.plugin.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-') {
            return Err(ManifestError::ValidationError(format!(
                "Invalid characters in plugin ID '{}'. Must contain only ascii alphanumeric, '.', '_', or '-'.",
                self.plugin.id
            )));
        }

        if self.plugin.name.trim().is_empty() {
            return Err(ManifestError::ValidationError(
                "Plugin name cannot be empty".to_string(),
            ));
        }

        if self.plugin.version.trim().is_empty() {
            return Err(ManifestError::ValidationError(
                "Plugin version cannot be empty".to_string(),
            ));
        }

        if self.resources.max_memory_mb == 0 {
            return Err(ManifestError::ValidationError(
                "max_memory_mb must be greater than 0".to_string(),
            ));
        }

        if self.resources.max_memory_mb > 2048 {
            return Err(ManifestError::ValidationError(
                "max_memory_mb cannot exceed 2048 MB (2 GB)".to_string(),
            ));
        }

        if self.resources.max_storage_mb > 10240 {
            return Err(ManifestError::ValidationError(
                "max_storage_mb cannot exceed 10240 MB (10 GB)".to_string(),
            ));
        }

        Ok(())
    }
}

impl fmt::Display for PluginManifest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} v{} by {} ({})",
            self.plugin.name,
            self.plugin.version,
            self.plugin.author,
            self.tier().display_label()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tier1_manifest() {
        let toml_str = r#"
[plugin]
id = "org.bitsr.quick_notes"
name = "Quick Notes"
version = "1.2.0"
author = "Accessibility Devs"
description = "Quickly take and speak accessible notes anywhere."
min_bit_sr_version = "0.1.0"

[resources]
max_memory_mb = 64
max_storage_mb = 25

[permissions]
storage = true
speech = { output = true, filter = false }
network = { access = true, allowed_domains = ["api.notesync.com"] }
hotkeys = ["SR + Shift + N"]
system_access = false
"#;

        let manifest = PluginManifest::from_toml_str(toml_str).expect("Valid manifest");
        assert_eq!(manifest.plugin.id, "org.bitsr.quick_notes");
        assert_eq!(manifest.tier(), ExtensionTier::Tier1Standard);
        assert!(manifest.permissions.storage);
        assert!(manifest.permissions.speech.output);
        assert!(!manifest.permissions.speech.filter);
        assert_eq!(manifest.permissions.network.allowed_domains, vec!["api.notesync.com"]);
        assert_eq!(manifest.resources.max_memory_mb, 64);
        assert_eq!(manifest.resources.max_storage_mb, 25);
    }

    #[test]
    fn test_parse_tier2_manifest() {
        let toml_str = r#"
[plugin]
id = "com.vendor.braille_display"
name = "Focus Braille Display Driver"
version = "2.0.0"
author = "Freedom Innovations"
description = "Vendor USB braille display hardware driver."

[resources]
max_memory_mb = 128

[permissions]
system_access = true
"#;

        let manifest = PluginManifest::from_toml_str(toml_str).expect("Valid manifest");
        assert_eq!(manifest.plugin.id, "com.vendor.braille_display");
        assert_eq!(manifest.tier(), ExtensionTier::Tier2SystemAccess);
        assert!(manifest.tier().is_system_access());
        assert_eq!(manifest.resources.max_memory_mb, 128);
        assert_eq!(manifest.resources.max_storage_mb, 25); // default
    }

    #[test]
    fn test_manifest_validation_failures() {
        let invalid_id = r#"
[plugin]
id = "bad id with spaces"
name = "Bad Plugin"
version = "1.0.0"
author = "Dev"
description = "Test"
"#;
        assert!(PluginManifest::from_toml_str(invalid_id).is_err());

        let zero_memory = r#"
[plugin]
id = "org.test.zero_mem"
name = "Zero Mem"
version = "1.0.0"
author = "Dev"
description = "Test"

[resources]
max_memory_mb = 0
"#;
        assert!(PluginManifest::from_toml_str(zero_memory).is_err());
    }
}
