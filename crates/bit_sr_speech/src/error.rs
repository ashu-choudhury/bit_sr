//! Error types for speech synthesis, driver management, and audio playback.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpeechError {
    /// The specified synthesizer driver is not registered or unavailable.
    SynthNotAvailable(String),

    /// The requested voice was not found on this synthesizer.
    VoiceNotFound(String),

    /// Error during text-to-speech rendering or speaking.
    SynthesisFailed(String),

    /// Audio output device or stream failure.
    AudioOutputError(String),

    /// Underlying COM or platform API error.
    PlatformError(String),

    /// General internal speech subsystem error.
    Internal(String),
}

impl fmt::Display for SpeechError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SynthNotAvailable(name) => write!(f, "Synthesizer driver '{}' not available", name),
            Self::VoiceNotFound(id) => write!(f, "Voice '{}' not found", id),
            Self::SynthesisFailed(msg) => write!(f, "Synthesis failed: {}", msg),
            Self::AudioOutputError(msg) => write!(f, "Audio output error: {}", msg),
            Self::PlatformError(msg) => write!(f, "Platform speech error: {}", msg),
            Self::Internal(msg) => write!(f, "Speech internal error: {}", msg),
        }
    }
}

impl std::error::Error for SpeechError {}

pub type Result<T> = std::result::Result<T, SpeechError>;
