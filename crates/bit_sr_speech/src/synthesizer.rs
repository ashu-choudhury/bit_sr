//! The Universal Synthesizer Driver Trait.
//! Modeled after NVDA's `synthDriverHandler.SynthDriver`.
//! All built-in drivers (SAPI5, eSpeak) and third-party WebAssembly extensions implement this trait.

use crate::error::Result;
use crate::voice::VoiceInfo;

/// Priority level for speech utterances.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpeechPriority {
    /// Interrupts any current speech immediately and speaks now.
    #[default]
    Now,

    /// Queued after current speech finishes.
    Next,

    /// Dropped if speech is currently ongoing (useful for progress bars or noisy repetitive events).
    OnlyIfIdle,
}

/// The universal interface for text-to-speech engines.
pub trait SynthesizerDriver: Send + Sync {
    /// Internal machine identifier (e.g. "sapi5", "espeak_ng", "wasm_piper").
    fn name(&self) -> &str;

    /// User-visible human-readable name (e.g. "Microsoft SAPI 5", "eSpeak NG").
    fn display_name(&self) -> &str;

    /// Lists all installed/available voices for this synthesizer.
    fn available_voices(&self) -> Vec<VoiceInfo>;

    /// Gets the currently active voice.
    fn current_voice(&self) -> Option<VoiceInfo>;

    /// Sets the active voice by ID.
    fn set_voice(&mut self, voice_id: &str) -> Result<()>;

    /// Synthesizes and speaks text.
    /// If `interrupt` is true, stops any in-progress speech immediately before starting.
    fn speak(&mut self, text: &str, interrupt: bool) -> Result<()>;

    /// Immediately stops all speech and purges audio buffers.
    fn stop(&mut self) -> Result<()>;

    /// Pauses current speech playback.
    fn pause(&mut self) -> Result<()>;

    /// Resumes paused speech playback.
    fn resume(&mut self) -> Result<()>;

    /// Sets speaking rate (speed), typically from -10 to 10 or 0 to 100.
    fn set_rate(&mut self, rate: i32) -> Result<()>;

    /// Gets current speaking rate.
    fn get_rate(&self) -> i32;

    /// Sets volume (0 to 100).
    fn set_volume(&mut self, volume: u16) -> Result<()>;

    /// Gets current volume (0 to 100).
    fn get_volume(&self) -> u16;

    /// Sets pitch (-10 to 10 or 0 to 100), if supported.
    fn set_pitch(&mut self, _pitch: i32) -> Result<()> {
        Ok(())
    }

    /// Gets current pitch.
    fn get_pitch(&self) -> i32 {
        0
    }
}
