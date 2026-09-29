//! Dynamic runtime capability and permission model.
//!
//! Provides granular capability management, explicit user trust tracking for Tier 2,
//! and live runtime revocation as specified in Section 6 and Section 9.C of the specification.

use crate::manifest::PluginManifest;
use serde::{Deserialize, Serialize};

/// Discrete permission identifiers that can be individually granted or revoked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PermissionType {
    /// Read/write access inside the virtualized `/data` scoped directory.
    Storage,
    /// Permission to produce speech output or audio sound cues.
    SpeechOutput,
    /// Permission to intercept and transform outgoing speech text.
    SpeechFilter,
    /// Permission to perform network HTTP/HTTPS requests.
    Network,
    /// Permission to register global or context-sensitive hotkeys.
    Hotkeys,
    /// Full unrestricted host bridge access (any DLL, CLI commands, hardware).
    SystemAccess,
    /// Read focused accessibility node name, role, states, and value.
    AccessibilityRead,
    /// Navigate parent, child, and sibling elements in the accessibility tree.
    AccessibilityWalk,
    /// Invoke accessible actions on UI elements.
    AccessibilityInteract,
}

impl PermissionType {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Storage => "Scoped Storage (/data)",
            Self::SpeechOutput => "Speech Output",
            Self::SpeechFilter => "Speech Text Transformation",
            Self::Network => "Network Access",
            Self::Hotkeys => "Keyboard Shortcuts",
            Self::SystemAccess => "Full System Access (Tier 2)",
            Self::AccessibilityRead => "Inspect Focused Elements",
            Self::AccessibilityWalk => "Walk Accessibility Tree",
            Self::AccessibilityInteract => "Interact with Accessible Controls",
        }
    }
}

/// The live set of granted permissions for an active extension instance.
///
/// Can be mutated dynamically at runtime by the user or security policies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrantedPermissions {
    pub storage: bool,
    pub speech_output: bool,
    pub speech_filter: bool,
    pub network: bool,
    pub allowed_domains: Vec<String>,
    pub hotkeys: Vec<String>,
    pub system_access: bool,
    pub accessibility_read: bool,
    pub accessibility_walk: bool,
    pub accessibility_interact: bool,
}

impl Default for GrantedPermissions {
    fn default() -> Self {
        Self {
            storage: false,
            speech_output: false,
            speech_filter: false,
            network: false,
            allowed_domains: Vec::new(),
            hotkeys: Vec::new(),
            system_access: false,
            accessibility_read: false,
            accessibility_walk: false,
            accessibility_interact: false,
        }
    }
}

impl GrantedPermissions {
    /// Initializes granted permissions from a declared manifest.
    ///
    /// For Tier 1 capabilities, declared permissions are granted.
    /// For Tier 2 `system_access`, explicit user trust (`grant_system_access: true`) is required.
    pub fn from_manifest(manifest: &PluginManifest, grant_system_access: bool) -> Self {
        let p = &manifest.permissions;
        Self {
            storage: p.storage,
            speech_output: p.speech.output,
            speech_filter: p.speech.filter,
            network: p.network.access,
            allowed_domains: p.network.allowed_domains.clone(),
            hotkeys: p.hotkeys.clone(),
            system_access: p.system_access && grant_system_access,
            accessibility_read: p.accessibility.read,
            accessibility_walk: p.accessibility.walk,
            accessibility_interact: p.accessibility.interact,
        }
    }

    /// Checks if a specific permission is currently granted.
    pub fn has_permission(&self, perm: PermissionType) -> bool {
        match perm {
            PermissionType::Storage => self.storage,
            PermissionType::SpeechOutput => self.speech_output,
            PermissionType::SpeechFilter => self.speech_filter,
            PermissionType::Network => self.network,
            PermissionType::Hotkeys => !self.hotkeys.is_empty(),
            PermissionType::SystemAccess => self.system_access,
            PermissionType::AccessibilityRead => self.accessibility_read,
            PermissionType::AccessibilityWalk => self.accessibility_walk,
            PermissionType::AccessibilityInteract => self.accessibility_interact,
        }
    }

    /// Grants a specific capability.
    pub fn grant(&mut self, perm: PermissionType) {
        match perm {
            PermissionType::Storage => self.storage = true,
            PermissionType::SpeechOutput => self.speech_output = true,
            PermissionType::SpeechFilter => self.speech_filter = true,
            PermissionType::Network => self.network = true,
            PermissionType::Hotkeys => {},
            PermissionType::SystemAccess => self.system_access = true,
            PermissionType::AccessibilityRead => self.accessibility_read = true,
            PermissionType::AccessibilityWalk => self.accessibility_walk = true,
            PermissionType::AccessibilityInteract => self.accessibility_interact = true,
        }
    }

    /// Revokes a specific capability immediately at runtime.
    pub fn revoke(&mut self, perm: PermissionType) {
        match perm {
            PermissionType::Storage => self.storage = false,
            PermissionType::SpeechOutput => self.speech_output = false,
            PermissionType::SpeechFilter => self.speech_filter = false,
            PermissionType::Network => self.network = false,
            PermissionType::Hotkeys => self.hotkeys.clear(),
            PermissionType::SystemAccess => self.system_access = false,
            PermissionType::AccessibilityRead => self.accessibility_read = false,
            PermissionType::AccessibilityWalk => self.accessibility_walk = false,
            PermissionType::AccessibilityInteract => self.accessibility_interact = false,
        }
    }

    /// Checks whether an outbound network destination is allowed under the current policy.
    pub fn can_access_domain(&self, domain: &str) -> bool {
        if !self.network {
            return false;
        }
        if self.allowed_domains.is_empty() {
            return false;
        }
        let domain_lower = domain.to_lowercase();
        self.allowed_domains
            .iter()
            .any(|allowed| allowed.to_lowercase() == domain_lower || domain_lower.ends_with(&format!(".{}", allowed.to_lowercase())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::PluginManifest;

    #[test]
    fn test_tier1_permissions_without_system_access() {
        let toml_str = r#"
[plugin]
id = "org.bitsr.test"
name = "Test"
version = "1.0.0"
author = "Dev"
description = "Desc"

[permissions]
storage = true
speech = { output = true, filter = false }
network = { access = true, allowed_domains = ["example.com"] }
system_access = true
"#;
        let manifest = PluginManifest::from_toml_str(toml_str).unwrap();
        // Even though manifest requested system_access = true, if user did NOT grant trust:
        let perms = GrantedPermissions::from_manifest(&manifest, false);
        assert!(perms.has_permission(PermissionType::Storage));
        assert!(perms.has_permission(PermissionType::SpeechOutput));
        assert!(!perms.has_permission(PermissionType::SpeechFilter));
        assert!(perms.has_permission(PermissionType::Network));
        assert!(!perms.has_permission(PermissionType::SystemAccess));
        assert!(perms.can_access_domain("example.com"));
        assert!(!perms.can_access_domain("malicious.com"));
    }

    #[test]
    fn test_tier2_permissions_with_user_trust() {
        let toml_str = r#"
[plugin]
id = "org.bitsr.test"
name = "Test"
version = "1.0.0"
author = "Dev"
description = "Desc"

[permissions]
system_access = true
"#;
        let manifest = PluginManifest::from_toml_str(toml_str).unwrap();
        let mut perms = GrantedPermissions::from_manifest(&manifest, true);
        assert!(perms.has_permission(PermissionType::SystemAccess));

        // Test runtime revocation
        perms.revoke(PermissionType::SystemAccess);
        assert!(!perms.has_permission(PermissionType::SystemAccess));
    }
}
