//! Accessible Settings UI Dashboard.
//!
//! Exposes configuration models and reactive handlers for the Slint settings dialog.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiSettings {
    pub locale_index: i32,
    pub sr_modifier_index: i32, // 0 = CapsLock, 1 = Insert, 2 = Both
    pub start_at_logon: bool,
    pub synth_driver_index: i32,
    pub voice_index: i32,
    pub speech_rate: f32,
    pub speech_volume: f32,
    pub speech_pitch: f32,
    pub echo_mode_index: i32, // 0 = Chars, 1 = Words, 2 = Both, 3 = None
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            locale_index: 0,
            sr_modifier_index: 0,
            start_at_logon: false,
            synth_driver_index: 0,
            voice_index: 0,
            speech_rate: 50.0,
            speech_volume: 80.0,
            speech_pitch: 50.0,
            echo_mode_index: 2,
        }
    }
}
