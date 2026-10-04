//! Error types for the Android platform crate.

use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Jni(String),
    NodeNotFound(u64),
    NotSupported(&'static str),
    Internal(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Jni(msg) => write!(f, "JNI bridge error: {msg}"),
            Self::NodeNotFound(id) => write!(f, "Node 0x{id:X} not found in spatial cache"),
            Self::NotSupported(feat) => write!(f, "Feature not supported on this Android API level: {feat}"),
            Self::Internal(msg) => write!(f, "Internal Android platform error: {msg}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<jni::errors::Error> for Error {
    fn from(e: jni::errors::Error) -> Self {
        Self::Jni(e.to_string())
    }
}
