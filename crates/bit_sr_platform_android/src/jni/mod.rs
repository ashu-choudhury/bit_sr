//! JNI Bridge Subsystem.
//! Connects AOSP `AccessibilityService` events and input down to the native Rust engine.

pub mod bridge;
pub mod converters;

use std::sync::atomic::AtomicI64;
use parking_lot::Mutex;

use crate::feedback::tts::AndroidTtsDriver;
use crate::input::touch::TouchStateMachine;
use crate::tree::cache::SpatialNodeCache;

/// Native Android Screen Reader Engine State.
pub struct AndroidEngineState {
    pub touch_machine: Mutex<TouchStateMachine>,
    pub node_cache: Mutex<SpatialNodeCache>,
    pub tts: Mutex<AndroidTtsDriver>,
    pub last_focused_node: AtomicI64,
}

impl AndroidEngineState {
    pub fn new() -> Self {
        Self {
            touch_machine: Mutex::new(TouchStateMachine::new()),
            node_cache: Mutex::new(SpatialNodeCache::new()),
            tts: Mutex::new(AndroidTtsDriver::new()),
            last_focused_node: AtomicI64::new(0),
        }
    }
}

impl Default for AndroidEngineState {
    fn default() -> Self {
        Self::new()
    }
}
