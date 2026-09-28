//! Pluggable Text-to-Speech Engine, Voice Registry, and Audio Output Pipeline for bit_sr.
//! Modeled after NVDA's `synthDriverHandler` and modern modular speech systems.

pub mod audio;
pub mod drivers;
pub mod error;
pub mod hub;
pub mod synthesizer;
pub mod voice;

pub use audio::{AudioChunk, AudioConsumer, AudioFormat};
pub use drivers::MockSynthesizer;
#[cfg(windows)]
pub use drivers::Sapi5Synthesizer;
pub use error::{Result, SpeechError};
pub use hub::SpeechHub;
pub use synthesizer::{SpeechPriority, SynthesizerDriver};
pub use voice::{VoiceGender, VoiceInfo};
