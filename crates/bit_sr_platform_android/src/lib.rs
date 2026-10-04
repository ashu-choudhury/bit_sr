//! Ultra-low-latency native Android accessibility platform backend for `bit_sr`.
//! Structured symmetrically with `bit_sr_platform_windows`.

pub mod apps;
pub mod error;
pub mod jni_bridge;
pub mod node_cache;
pub mod node_mapper;
pub mod speech_bridge;
pub mod touch_machine;

pub use apps::{AndroidAppType, AndroidChromeFilter, SoftKeyboardFilter, SystemUiFilter};
pub use error::{Error, Result};
pub use node_cache::{CachedNode, SpatialNodeCache};
pub use node_mapper::map_class_name_to_role;
pub use speech_bridge::AndroidTtsDriver;
pub use touch_machine::{TouchResult, TouchStateMachine};

use bit_sr_core::events::AccessibilityEvent;
use crossbeam_channel::Sender;
use std::sync::Arc;
use parking_lot::Mutex;

/// Master platform coordinator for Android.
/// Coordinates the native touch state machine, spatial node cache,
/// and JNI accessibility pipeline.
pub struct AndroidPlatform {
    pub node_cache: Arc<Mutex<SpatialNodeCache>>,
    pub touch_machine: Arc<Mutex<TouchStateMachine>>,
    pub event_tx: Sender<AccessibilityEvent>,
}

impl AndroidPlatform {
    /// Initializes the Android platform coordinator.
    pub fn start(event_tx: Sender<AccessibilityEvent>) -> Result<Self> {
        log::info!("Starting Android accessibility platform...");
        let node_cache = Arc::new(Mutex::new(SpatialNodeCache::new()));
        let touch_machine = Arc::new(Mutex::new(TouchStateMachine::new()));

        Ok(Self {
            node_cache,
            touch_machine,
            event_tx,
        })
    }

    /// Access the in-memory spatial cache.
    pub fn cache(&self) -> &Arc<Mutex<SpatialNodeCache>> {
        &self.node_cache
    }

    /// Access the touch state machine.
    pub fn touch(&self) -> &Arc<Mutex<TouchStateMachine>> {
        &self.touch_machine
    }
}
