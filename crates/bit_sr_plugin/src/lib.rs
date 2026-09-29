//! WebAssembly Extension Ecosystem for `bit_sr`.
//!
//! Provides a capability-based extension runtime built on Wasmtime,
//! supporting Tier 1 sandboxed extensions and Tier 2 system extensions.

pub mod host_bridge;
pub mod limiter;
pub mod manifest;
pub mod manager;
pub mod package;
pub mod permissions;
pub mod runtime;
pub mod storage;

pub use host_bridge::{CommandOutput, HostBridgeError, PluginContext, PluginSpeechSink};
pub use limiter::ExtensionMemoryLimiter;
pub use manifest::{
    AccessibilityPermissionConfig, ExtensionTier, ManifestError, NetworkPermissionConfig,
    PermissionConfig, PluginManifest, PluginMetadata, ResourceConfig, SpeechPermissionConfig,
};
pub use manager::{ExtensionStatus, PluginError, PluginInfo, PluginManager, PluginRecord};
pub use package::{PackageError, PluginPackage};
pub use permissions::{GrantedPermissions, PermissionType};
pub use runtime::{PluginInstance, PluginState, WasmEngine};
pub use storage::{ScopedStorage, StorageError};

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
