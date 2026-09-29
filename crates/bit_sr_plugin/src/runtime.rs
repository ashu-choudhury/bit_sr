//! WebAssembly Wasmtime runtime execution, epoch deadline watchdog,
//! and host bridge registration.

use crate::host_bridge::{HostBridgeError, PluginContext};
use crate::limiter::ExtensionMemoryLimiter;
use crate::manifest::PluginManifest;
use crate::package::PluginPackage;
use crate::permissions::PermissionType;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use wasmtime::{
    Caller, Config, Engine, Instance, Linker, Module, OptLevel, Store, Trap,
};
use wasmtime_wasi::p1::{self, WasiP1Ctx};
use wasmtime_wasi::{FsPerms, WasiCtxBuilder};

/// State encapsulated within each Wasmtime `Store` for an extension instance.
pub struct PluginState {
    /// WASI Preview 1 context.
    pub wasi: WasiP1Ctx,
    /// Hardware-clamped memory limiter.
    pub limiter: ExtensionMemoryLimiter,
    /// Host gatekeeper context (capabilities, scoped storage, dynamic libraries).
    pub context: PluginContext,
}

/// WebAssembly runtime execution engine for `bit_sr`.
#[derive(Clone)]
pub struct WasmEngine {
    engine: Engine,
    stop_epoch_ticker: Arc<AtomicBool>,
}

impl WasmEngine {
    /// Creates a new engine configured with epoch interruption and Cranelift optimizations.
    pub fn new() -> Result<Self, wasmtime::Error> {
        let mut config = Config::new();
        config.epoch_interruption(true);
        config.cranelift_opt_level(OptLevel::Speed);

        let engine = Engine::new(&config)?;
        let stop_epoch_ticker = Arc::new(AtomicBool::new(false));

        // Start background epoch ticker for anti-freeze watchdog (ticks every 50ms)
        let ticker_engine = engine.clone();
        let ticker_stop = stop_epoch_ticker.clone();
        thread::Builder::new()
            .name("bit_sr-epoch-ticker".to_string())
            .spawn(move || {
                while !ticker_stop.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_millis(50));
                    ticker_engine.increment_epoch();
                }
            })
            .ok();

        Ok(Self {
            engine,
            stop_epoch_ticker,
        })
    }

    /// Access the underlying Wasmtime engine.
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Increments the epoch counter across the engine to enforce deadlines.
    pub fn tick_epoch(&self) {
        self.engine.increment_epoch();
    }

    /// Builds a configured linker containing WASI and bit_sr host bridge functions.
    pub fn build_linker(&self) -> Result<Linker<PluginState>, wasmtime::Error> {
        let mut linker: Linker<PluginState> = Linker::new(&self.engine);

        // 1. Link standard WASI Preview 1
        p1::add_to_linker_sync(&mut linker, |state: &mut PluginState| &mut state.wasi)?;

        // 2. Link bit_sr host bridge APIs
        register_host_apis(&mut linker)?;

        Ok(linker)
    }

    /// Instantiates an extension from an unpacked package.
    pub fn instantiate_plugin(
        &self,
        package: &PluginPackage,
        context: PluginContext,
    ) -> Result<PluginInstance, wasmtime::Error> {
        let linker = self.build_linker()?;
        let module = Module::new(&self.engine, &package.wasm_bytes)?;

        // Build WASI context with preopened /data directory
        let mut wasi_builder = WasiCtxBuilder::new();
        wasi_builder.inherit_stdout().inherit_stderr();

        // Ensure host /data directory exists and mount it as /data
        if let Ok(host_dir) = context.storage.ensure_dir() {
            let _ = wasi_builder.preopened_dir(host_dir, "/data", FsPerms::ReadWrite);
        }

        let wasi_p1 = wasi_builder.build_p1();
        let limiter = ExtensionMemoryLimiter::from_mb(package.manifest.resources.max_memory_mb);

        let state = PluginState {
            wasi: wasi_p1,
            limiter,
            context,
        };

        let mut store = Store::new(&self.engine, state);
        // Apply hardware-level RAM limiter
        store.limiter(|s| &mut s.limiter);

        let instance = linker.instantiate(&mut store, &module)?;

        Ok(PluginInstance {
            id: package.manifest.plugin.id.clone(),
            manifest: package.manifest.clone(),
            store: Mutex::new(store),
            instance,
            engine: self.clone(),
        })
    }
}

impl Drop for WasmEngine {
    fn drop(&mut self) {
        self.stop_epoch_ticker.store(true, Ordering::Relaxed);
    }
}

/// An active, instantiated WebAssembly extension.
pub struct PluginInstance {
    pub id: String,
    pub manifest: PluginManifest,
    pub store: Mutex<Store<PluginState>>,
    pub instance: Instance,
    pub engine: WasmEngine,
}

impl PluginInstance {
    /// Calls the optional `bit_sr_init` export of the extension.
    pub fn init(&self) -> Result<(), HostBridgeError> {
        let mut store = self.store.lock().unwrap();
        // Compute deadline ticks (each tick is ~50ms)
        let timeout_ms = self.manifest.resources.execution_timeout_ms;
        let ticks = (timeout_ms / 50).max(1);
        store.set_epoch_deadline(ticks);

        if let Ok(func) = self.instance.get_typed_func::<(), i32>(&mut *store, "bit_sr_init") {
            let res = func.call(&mut *store, ());
            match res {
                Ok(code) => {
                    if code == 0 {
                        Ok(())
                    } else {
                        Err(HostBridgeError::ExecutionFailed(
                            "bit_sr_init".to_string(),
                            format!("Init returned non-zero code {code}"),
                        ))
                    }
                }
                Err(err) => {
                    if is_epoch_trap(&err) {
                        Err(HostBridgeError::ExecutionFailed(
                            "bit_sr_init".to_string(),
                            format!("Extension '{}' timed out during init and was halted", self.id),
                        ))
                    } else {
                        Err(HostBridgeError::ExecutionFailed(
                            "bit_sr_init".to_string(),
                            err.to_string(),
                        ))
                    }
                }
            }
        } else {
            Ok(())
        }
    }

    /// Calls the optional `bit_sr_on_speech_filter` export.
    pub fn on_speech_filter(&self, text: &str) -> Result<Option<String>, HostBridgeError> {
        let mut store = self.store.lock().unwrap();
        let timeout_ms = self.manifest.resources.execution_timeout_ms;
        let ticks = (timeout_ms / 50).max(1);
        store.set_epoch_deadline(ticks);

        // Check if export exists
        let func = match self.instance.get_typed_func::<(i32, i32), i64>(&mut *store, "bit_sr_on_speech_filter") {
            Ok(f) => f,
            Err(_) => return Ok(None),
        };

        // Write input text into guest memory via export alloc or guest memory write
        let (ptr, len) = match allocate_and_write_guest_str(&self.instance, &mut *store, text) {
            Ok(loc) => loc,
            Err(_) => return Ok(None),
        };

        let result = func.call(&mut *store, (ptr, len));
        match result {
            Ok(res_val) => {
                if res_val <= 0 {
                    Ok(None)
                } else {
                    let out_ptr = (res_val >> 32) as i32;
                    let out_len = (res_val & 0xFFFFFFFF) as i32;
                    let transformed = read_guest_string(&self.instance, &mut *store, out_ptr, out_len).ok();
                    Ok(transformed)
                }
            }
            Err(err) => {
                if is_epoch_trap(&err) {
                    log::warn!("Extension '{}' timed out during on_speech_filter", self.id);
                }
                Ok(None) // Fail safe: never silence or break speech
            }
        }
    }
}

/// Helper to detect if an error was caused by Wasmtime's epoch interruption deadline.
fn is_epoch_trap(err: &wasmtime::Error) -> bool {
    if let Some(trap) = err.downcast_ref::<Trap>() {
        matches!(trap, Trap::Interrupt)
    } else {
        err.to_string().contains("interrupt")
    }
}

/// Allocates memory in guest and writes string if `bit_sr_alloc` export is provided.
fn allocate_and_write_guest_str(
    instance: &Instance,
    store: &mut Store<PluginState>,
    text: &str,
) -> Result<(i32, i32), wasmtime::Error> {
    let bytes = text.as_bytes();
    let len = bytes.len() as i32;

    if let Ok(alloc_func) = instance.get_typed_func::<i32, i32>(&mut *store, "bit_sr_alloc") {
        let ptr = alloc_func.call(&mut *store, len)?;
        let memory = instance
            .get_memory(&mut *store, "memory")
            .ok_or_else(|| wasmtime::Error::msg("Missing 'memory' export"))?;
        memory.write(&mut *store, ptr as usize, bytes)?;
        Ok((ptr, len))
    } else {
        Err(wasmtime::Error::msg("Guest does not export bit_sr_alloc"))
    }
}

/// Reads a UTF-8 string from guest linear memory.
fn read_guest_string(
    instance: &Instance,
    store: &mut Store<PluginState>,
    ptr: i32,
    len: i32,
) -> Result<String, wasmtime::Error> {
    let memory = instance
        .get_memory(&mut *store, "memory")
        .ok_or_else(|| wasmtime::Error::msg("Missing 'memory' export"))?;
    let data = memory.data(&*store);
    let start = ptr as usize;
    let end = start.saturating_add(len as usize);
    if end > data.len() {
        return Err(wasmtime::Error::msg("Memory read out of bounds"));
    }
    let slice = &data[start..end];
    let s = std::str::from_utf8(slice)
        .map_err(|e| wasmtime::Error::msg(format!("Invalid UTF-8: {e}")))?;
    Ok(s.to_string())
}

/// Registers `bit_sr` custom host APIs with the linker.
fn register_host_apis(linker: &mut Linker<PluginState>) -> Result<(), wasmtime::Error> {
    // -------------------------------------------------------------
    // Namespace: bit_sr_sys (Tier 2 guarded APIs)
    // -------------------------------------------------------------
    linker.func_wrap(
        "bit_sr_sys",
        "load_library",
        |mut caller: Caller<'_, PluginState>, path_ptr: i32, path_len: i32| -> i64 {
            let path = match read_caller_string(&mut caller, path_ptr, path_len) {
                Ok(p) => p,
                Err(_) => return -1,
            };
            match caller.data().context.sys_load_library(&path) {
                Ok(handle) => handle as i64,
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "bit_sr_sys",
        "unload_library",
        |caller: Caller<'_, PluginState>, handle: i64| -> i32 {
            match caller.data().context.sys_unload_library(handle as u64) {
                Ok(()) => 0,
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "bit_sr_sys",
        "execute_command",
        |mut caller: Caller<'_, PluginState>, cmd_ptr: i32, cmd_len: i32| -> i32 {
            let cmd = match read_caller_string(&mut caller, cmd_ptr, cmd_len) {
                Ok(c) => c,
                Err(_) => return -1,
            };
            match caller.data().context.sys_execute_command(&cmd, &[]) {
                Ok(out) => out.exit_code,
                Err(_) => -1,
            }
        },
    )?;

    // -------------------------------------------------------------
    // Namespace: bit_sr_speech (Tier 1 capability APIs)
    // -------------------------------------------------------------
    linker.func_wrap(
        "bit_sr_speech",
        "speak",
        |mut caller: Caller<'_, PluginState>, text_ptr: i32, text_len: i32, interrupt: i32| -> i32 {
            let text = match read_caller_string(&mut caller, text_ptr, text_len) {
                Ok(t) => t,
                Err(_) => return -1,
            };
            match caller.data().context.speech_speak(&text, interrupt != 0) {
                Ok(()) => 0,
                Err(_) => -1,
            }
        },
    )?;

    linker.func_wrap(
        "bit_sr_speech",
        "stop",
        |caller: Caller<'_, PluginState>| -> i32 {
            match caller.data().context.speech_stop() {
                Ok(()) => 0,
                Err(_) => -1,
            }
        },
    )?;

    // -------------------------------------------------------------
    // Namespace: bit_sr_storage
    // -------------------------------------------------------------
    linker.func_wrap(
        "bit_sr_storage",
        "usage",
        |caller: Caller<'_, PluginState>| -> i64 {
            caller
                .data()
                .context
                .storage
                .current_usage_bytes()
                .map(|b| b as i64)
                .unwrap_or(-1)
        },
    )?;

    linker.func_wrap(
        "bit_sr_storage",
        "limit",
        |caller: Caller<'_, PluginState>| -> i64 {
            caller.data().context.storage.max_storage_bytes() as i64
        },
    )?;

    // -------------------------------------------------------------
    // Namespace: bit_sr_log
    // -------------------------------------------------------------
    linker.func_wrap(
        "bit_sr_log",
        "log",
        |mut caller: Caller<'_, PluginState>, level: i32, msg_ptr: i32, msg_len: i32| {
            if let Ok(msg) = read_caller_string(&mut caller, msg_ptr, msg_len) {
                caller.data().context.host_log(level as u32, &msg);
            }
        },
    )?;

    // -------------------------------------------------------------
    // Namespace: bit_sr_permission
    // -------------------------------------------------------------
    linker.func_wrap(
        "bit_sr_permission",
        "has_permission",
        |caller: Caller<'_, PluginState>, perm_id: i32| -> i32 {
            let perm = match perm_id {
                1 => PermissionType::Storage,
                2 => PermissionType::SpeechOutput,
                3 => PermissionType::SpeechFilter,
                4 => PermissionType::Network,
                5 => PermissionType::SystemAccess,
                6 => PermissionType::AccessibilityRead,
                7 => PermissionType::AccessibilityWalk,
                8 => PermissionType::AccessibilityInteract,
                _ => return 0,
            };
            let has = caller
                .data()
                .context
                .permissions
                .read()
                .map(|p| p.has_permission(perm))
                .unwrap_or(false);
            if has { 1 } else { 0 }
        },
    )?;

    Ok(())
}

/// Reads a UTF-8 string from a caller's memory.
fn read_caller_string(
    caller: &mut Caller<'_, PluginState>,
    ptr: i32,
    len: i32,
) -> Result<String, wasmtime::Error> {
    let memory = caller
        .get_export("memory")
        .and_then(|e| e.into_memory())
        .ok_or_else(|| wasmtime::Error::msg("Missing 'memory' export"))?;
    let data = memory.data(&*caller);
    let start = ptr as usize;
    let end = start.saturating_add(len as usize);
    if end > data.len() {
        return Err(wasmtime::Error::msg("Memory read out of bounds"));
    }
    let slice = &data[start..end];
    let s = std::str::from_utf8(slice)
        .map_err(|e| wasmtime::Error::msg(format!("Invalid UTF-8: {e}")))?;
    Ok(s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::GrantedPermissions;
    use std::sync::RwLock;
    use tempfile::tempdir;

    #[test]
    fn test_wasm_engine_initialization_and_epoch_tick() {
        let engine = WasmEngine::new().expect("Engine should initialize");
        engine.tick_epoch();
    }

    #[test]
    fn test_instantiate_minimal_wasm_module() {
        let tmp = tempdir().unwrap();
        let engine = WasmEngine::new().unwrap();

        let manifest_toml = r#"
[plugin]
id = "org.bitsr.minimal"
name = "Minimal Plugin"
version = "0.1.0"
author = "Dev"
description = "Test"

[resources]
max_memory_mb = 16
max_storage_mb = 5
"#;
        let manifest = PluginManifest::from_toml_str(manifest_toml).unwrap();

        // Minimal valid wasm module: (module (func (export "bit_sr_init") (result i32) (i32.const 0)))
        let wat = r#"
            (module
                (memory (export "memory") 1)
                (func (export "bit_sr_init") (result i32)
                    i32.const 0
                )
            )
        "#;
        let wasm_bytes = wat::parse_str(wat).expect("Parse WAT");

        let package = PluginPackage::new(manifest.clone(), wasm_bytes, None);
        let perms = Arc::new(RwLock::new(GrantedPermissions::from_manifest(&manifest, false)));
        let context = PluginContext::new(manifest, perms, tmp.path().to_path_buf(), None);

        let instance = engine
            .instantiate_plugin(&package, context)
            .expect("Plugin instantiates cleanly");

        assert!(instance.init().is_ok());
    }

    #[test]
    fn test_epoch_interruption_anti_freeze_timeout() {
        let tmp = tempdir().unwrap();
        let engine = WasmEngine::new().unwrap();

        let manifest_toml = r#"
[plugin]
id = "org.bitsr.infinite_loop"
name = "Looping Plugin"
version = "0.1.0"
author = "Dev"
description = "Test infinite loop"

[resources]
max_memory_mb = 16
execution_timeout_ms = 50
"#;
        let manifest = PluginManifest::from_toml_str(manifest_toml).unwrap();

        // Infinite loop in Wasm: (loop (br 0))
        let wat = r#"
            (module
                (memory (export "memory") 1)
                (func (export "bit_sr_init") (result i32)
                    (loop (br 0))
                    i32.const 0
                )
            )
        "#;
        let wasm_bytes = wat::parse_str(wat).expect("Parse WAT");
        let package = PluginPackage::new(manifest.clone(), wasm_bytes, None);
        let perms = Arc::new(RwLock::new(GrantedPermissions::from_manifest(&manifest, false)));
        let context = PluginContext::new(manifest, perms, tmp.path().to_path_buf(), None);

        let instance = engine
            .instantiate_plugin(&package, context)
            .expect("Plugin instantiates");

        // The init call should trigger epoch trap without hanging the thread or crashing
        let err = instance.init().unwrap_err();
        assert!(matches!(err, HostBridgeError::ExecutionFailed(_, _)));
    }
}
