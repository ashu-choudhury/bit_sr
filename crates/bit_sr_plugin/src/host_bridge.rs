//! Host Gatekeeper & System Bridge.
//!
//! Provides the secure host bridge exposed to WebAssembly extension modules.
//! Enforces the two-tier security model:
//! - Tier 1 calls (speech, storage, logging) are checked against granular capabilities.
//! - Tier 2 calls (arbitrary DLL loading, system CLI execution, raw hardware)
//!   are strictly gated by `has_system_access` with live runtime revocation support.

use crate::manifest::PluginManifest;
use crate::permissions::{GrantedPermissions, PermissionType};
use crate::storage::ScopedStorage;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

/// Errors returned by the host bridge.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HostBridgeError {
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Failed to load dynamic library '{0}': {1}")]
    LoadFailed(String, String),
    #[error("Failed to execute system command '{0}': {1}")]
    ExecutionFailed(String, String),
    #[error("Dynamic library handle {0} not found")]
    InvalidHandle(u64),
    #[error("Storage quota exceeded: {0}")]
    StorageQuotaExceeded(String),
}

/// Output captured from a system command executed on behalf of a Tier 2 extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

impl From<std::process::Output> for CommandOutput {
    fn from(output: std::process::Output) -> Self {
        Self {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            success: output.status.success(),
        }
    }
}

/// Interface for speech output sink handling speech requests from extensions.
pub trait PluginSpeechSink: Send + Sync {
    /// Speak text with optional interruption of previous speech.
    fn speak(&self, text: &str, interrupt: bool);
    /// Immediately silence speech.
    fn stop(&self);
}

/// Execution context stored within the Wasmtime `Store` for each extension.
pub struct PluginContext {
    /// Extension identifier.
    pub id: String,
    /// Extension manifest.
    pub manifest: PluginManifest,
    /// Currently granted permissions (thread-safe, dynamically toggleable at runtime).
    pub permissions: Arc<RwLock<GrantedPermissions>>,
    /// Scoped storage instance.
    pub storage: ScopedStorage,
    /// Loaded dynamic libraries for Tier 2 extensions.
    pub loaded_libraries: Arc<Mutex<HashMap<u64, libloading::Library>>>,
    /// Handle allocator for loaded libraries.
    pub next_lib_handle: Arc<AtomicU64>,
    /// Optional sink for routing speech output.
    pub speech_sink: Option<Arc<dyn PluginSpeechSink>>,
}

impl PluginContext {
    /// Creates a new plugin context.
    pub fn new(
        manifest: PluginManifest,
        permissions: Arc<RwLock<GrantedPermissions>>,
        base_storage_dir: PathBuf,
        speech_sink: Option<Arc<dyn PluginSpeechSink>>,
    ) -> Self {
        let storage = ScopedStorage::new(
            &manifest.plugin.id,
            base_storage_dir,
            manifest.resources.max_storage_mb,
        );
        Self {
            id: manifest.plugin.id.clone(),
            manifest,
            permissions,
            storage,
            loaded_libraries: Arc::new(Mutex::new(HashMap::new())),
            next_lib_handle: Arc::new(AtomicU64::new(1)),
            speech_sink,
        }
    }

    /// Whether this extension currently has system access granted.
    pub fn has_system_access(&self) -> bool {
        self.permissions
            .read()
            .map(|p| p.has_permission(PermissionType::SystemAccess))
            .unwrap_or(false)
    }

    /// Host Gatekeeper: Load an arbitrary dynamic library (Tier 2 only).
    pub fn sys_load_library(&self, lib_path: &str) -> Result<u64, HostBridgeError> {
        if !self.has_system_access() {
            log::error!(
                "Extension '{}' attempted unauthorized DLL load: {}",
                self.id,
                lib_path
            );
            return Err(HostBridgeError::PermissionDenied(format!(
                "Extension '{}' lacks Tier 2 system_access permission to load '{}'",
                self.id, lib_path
            )));
        }

        log::info!("Extension '{}' loading dynamic library: {}", self.id, lib_path);
        let lib = unsafe { libloading::Library::new(lib_path) }
            .map_err(|e| HostBridgeError::LoadFailed(lib_path.to_string(), e.to_string()))?;

        let handle = self.next_lib_handle.fetch_add(1, Ordering::SeqCst);
        let mut libs = self.loaded_libraries.lock().unwrap();
        libs.insert(handle, lib);
        Ok(handle)
    }

    /// Host Gatekeeper: Unload a previously loaded library.
    pub fn sys_unload_library(&self, handle: u64) -> Result<(), HostBridgeError> {
        if !self.has_system_access() {
            return Err(HostBridgeError::PermissionDenied(
                "System access permission required to unload libraries".to_string(),
            ));
        }

        let mut libs = self.loaded_libraries.lock().unwrap();
        if libs.remove(&handle).is_some() {
            log::info!("Extension '{}' unloaded library handle {}", self.id, handle);
            Ok(())
        } else {
            Err(HostBridgeError::InvalidHandle(handle))
        }
    }

    /// Host Gatekeeper: Execute a system command (Tier 2 only).
    pub fn sys_execute_command(
        &self,
        command: &str,
        args: &[String],
    ) -> Result<CommandOutput, HostBridgeError> {
        if !self.has_system_access() {
            log::error!(
                "Extension '{}' attempted unauthorized command execution: {} {:?}",
                self.id,
                command,
                args
            );
            return Err(HostBridgeError::PermissionDenied(format!(
                "Extension '{}' lacks Tier 2 system_access permission to execute commands",
                self.id
            )));
        }

        log::info!(
            "Extension '{}' executing system command: {} {:?}",
            self.id,
            command,
            args
        );

        let output = Command::new(command)
            .args(args)
            .output()
            .map_err(|e| HostBridgeError::ExecutionFailed(command.to_string(), e.to_string()))?;

        Ok(CommandOutput::from(output))
    }

    /// Host Speech Output (Tier 1 capability).
    pub fn speech_speak(&self, text: &str, interrupt: bool) -> Result<(), HostBridgeError> {
        let has_speech = self
            .permissions
            .read()
            .map(|p| p.has_permission(PermissionType::SpeechOutput))
            .unwrap_or(false);

        if !has_speech {
            return Err(HostBridgeError::PermissionDenied(format!(
                "Extension '{}' does not have speech:output permission",
                self.id
            )));
        }

        if let Some(sink) = &self.speech_sink {
            sink.speak(text, interrupt);
        }
        Ok(())
    }

    /// Host Speech Stop (Tier 1 capability).
    pub fn speech_stop(&self) -> Result<(), HostBridgeError> {
        let has_speech = self
            .permissions
            .read()
            .map(|p| p.has_permission(PermissionType::SpeechOutput))
            .unwrap_or(false);

        if !has_speech {
            return Err(HostBridgeError::PermissionDenied(format!(
                "Extension '{}' does not have speech:output permission",
                self.id
            )));
        }

        if let Some(sink) = &self.speech_sink {
            sink.stop();
        }
        Ok(())
    }

    /// Host Log function.
    pub fn host_log(&self, level: u32, message: &str) {
        match level {
            1 => log::error!("[Plugin: {}] {}", self.id, message),
            2 => log::warn!("[Plugin: {}] {}", self.id, message),
            3 => log::info!("[Plugin: {}] {}", self.id, message),
            4 => log::debug!("[Plugin: {}] {}", self.id, message),
            _ => log::trace!("[Plugin: {}] {}", self.id, message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    struct TestSpeechSink {
        messages: Arc<Mutex<Vec<(String, bool)>>>,
    }

    impl PluginSpeechSink for TestSpeechSink {
        fn speak(&self, text: &str, interrupt: bool) {
            self.messages.lock().unwrap().push((text.to_string(), interrupt));
        }
        fn stop(&self) {}
    }

    #[test]
    fn test_tier1_speech_allowed_and_revoked() {
        let tmp = tempdir().unwrap();
        let toml_str = r#"
[plugin]
id = "org.bitsr.speech_test"
name = "Speech Test"
version = "1.0.0"
author = "Dev"
description = "Speech Test"

[permissions]
speech = { output = true, filter = false }
"#;
        let manifest = PluginManifest::from_toml_str(toml_str).unwrap();
        let perms = Arc::new(RwLock::new(GrantedPermissions::from_manifest(&manifest, false)));
        let messages = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::new(TestSpeechSink {
            messages: messages.clone(),
        });

        let ctx = PluginContext::new(manifest, perms.clone(), tmp.path().to_path_buf(), Some(sink));

        // Speech output works
        assert!(ctx.speech_speak("Hello world", false).is_ok());
        assert_eq!(messages.lock().unwrap().len(), 1);

        // Revoke speech output dynamically at runtime
        perms.write().unwrap().revoke(PermissionType::SpeechOutput);

        // Speech output should now be denied
        let err = ctx.speech_speak("Should fail", false).unwrap_err();
        assert!(matches!(err, HostBridgeError::PermissionDenied(_)));
        assert_eq!(messages.lock().unwrap().len(), 1); // Not added
    }

    #[test]
    fn test_tier2_command_gated_by_trust() {
        let tmp = tempdir().unwrap();
        let toml_str = r#"
[plugin]
id = "org.bitsr.sys_test"
name = "Sys Test"
version = "1.0.0"
author = "Dev"
description = "Sys Test"

[permissions]
system_access = true
"#;
        let manifest = PluginManifest::from_toml_str(toml_str).unwrap();

        // 1. Without trust granted
        let perms_untrusted = Arc::new(RwLock::new(GrantedPermissions::from_manifest(&manifest, false)));
        let ctx_untrusted = PluginContext::new(
            manifest.clone(),
            perms_untrusted,
            tmp.path().to_path_buf(),
            None,
        );

        let err = ctx_untrusted
            .sys_execute_command("cmd.exe", &["/C".into(), "echo 1".into()])
            .unwrap_err();
        assert!(matches!(err, HostBridgeError::PermissionDenied(_)));

        // 2. With trust granted
        let perms_trusted = Arc::new(RwLock::new(GrantedPermissions::from_manifest(&manifest, true)));
        let ctx_trusted = PluginContext::new(
            manifest,
            perms_trusted.clone(),
            tmp.path().to_path_buf(),
            None,
        );

        #[cfg(windows)]
        {
            let res = ctx_trusted
                .sys_execute_command("cmd.exe", &["/C".into(), "echo test_ok".into()]);
            assert!(res.is_ok());
            let out = res.unwrap();
            assert!(out.stdout.contains("test_ok"));
        }

        // 3. Revoke trust at runtime
        perms_trusted.write().unwrap().revoke(PermissionType::SystemAccess);
        let err2 = ctx_trusted
            .sys_execute_command("cmd.exe", &["/C".into(), "echo 2".into()])
            .unwrap_err();
        assert!(matches!(err2, HostBridgeError::PermissionDenied(_)));
    }
}
