//! Android accessibility platform error types and results.

use std::fmt;

/// Platform-specific error type for Android.
#[derive(Debug)]
pub enum Error {
    /// JNI operation failed.
    Jni(String),
    /// Node was not found in spatial cache.
    NodeNotFound(u64),
    /// Invalid window id.
    InvalidWindow(i32),
    /// Speech synthesizer error.
    Speech(String),
    /// Haptics hardware unavailable.
    HapticsUnavailable,
    /// Channel disconnected.
    ChannelDisconnected,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Jni(msg) => write!(f, "Android JNI error: {}", msg),
            Self::NodeNotFound(id) => write!(f, "Android node 0x{:X} not found in spatial cache", id),
            Self::InvalidWindow(id) => write!(f, "Invalid Android window ID: {}", id),
            Self::Speech(msg) => write!(f, "Android speech error: {}", msg),
            Self::HapticsUnavailable => write!(f, "Android haptics vibrator unavailable"),
            Self::ChannelDisconnected => write!(f, "Android accessibility event channel disconnected"),
        }
    }
}

impl std::error::Error for Error {}

impl From<jni::errors::Error> for Error {
    fn from(err: jni::errors::Error) -> Self {
        Self::Jni(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
