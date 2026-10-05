//! Android Feedback Subsystem: Text-to-Speech, Low-Latency Earcons, and Haptics.

pub mod earcons;
pub mod haptics;
pub mod tts;

pub use earcons::EarconType;
pub use haptics::HapticEffect;
pub use tts::{clear_speech_callback, set_speech_callback, AndroidTtsDriver};
