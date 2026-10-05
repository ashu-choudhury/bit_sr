//! Master Android Platform Coordinator.
//! Coordinates touch state machine, spatial node cache, feedback, and accessibility event dispatch.

use std::sync::Arc;
use parking_lot::Mutex;
use crossbeam_channel::Sender;

use bit_sr_core::events::AccessibilityEvent;
use crate::error::Result;
use crate::feedback::tts::AndroidTtsDriver;
use crate::input::touch::TouchStateMachine;
use crate::tree::cache::SpatialNodeCache;

/// Master platform coordinator for Android.
pub struct AndroidPlatform {
    pub node_cache: Arc<Mutex<SpatialNodeCache>>,
    pub touch_machine: Arc<Mutex<TouchStateMachine>>,
    pub tts: Arc<Mutex<AndroidTtsDriver>>,
    pub event_tx: Sender<AccessibilityEvent>,
}

impl AndroidPlatform {
    /// Initializes the Android platform coordinator.
    pub fn start(event_tx: Sender<AccessibilityEvent>) -> Result<Self> {
        log::info!("Starting Android accessibility platform coordinator...");
        let node_cache = Arc::new(Mutex::new(SpatialNodeCache::new()));
        let touch_machine = Arc::new(Mutex::new(TouchStateMachine::new()));
        let tts = Arc::new(Mutex::new(AndroidTtsDriver::new()));

        Ok(Self {
            node_cache,
            touch_machine,
            tts,
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

    /// Access the TTS driver.
    pub fn tts(&self) -> &Arc<Mutex<AndroidTtsDriver>> {
        &self.tts
    }
}
