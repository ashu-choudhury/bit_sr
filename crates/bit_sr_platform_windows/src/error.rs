//! Error types for the Windows platform crate.

use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Windows(windows::core::Error),
    Win32(u32),
    ComInitializationFailed(i32),
    HookInstallationFailed(&'static str),
    Timeout(&'static str),
    ElementDead,
    NotSupported(&'static str),
    Internal(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Windows(e) => write!(f, "Windows API error: {e}"),
            Self::Win32(code) => write!(f, "Win32 error code: {code}"),
            Self::ComInitializationFailed(hr) => write!(f, "COM CoInitializeEx failed with HRESULT: 0x{hr:X}"),
            Self::HookInstallationFailed(name) => write!(f, "Failed to install Windows hook: {name}"),
            Self::Timeout(op) => write!(f, "Operation timed out: {op}"),
            Self::ElementDead => write!(f, "Accessible element is no longer valid or has died"),
            Self::NotSupported(feature) => write!(f, "Feature not supported: {feature}"),
            Self::Internal(msg) => write!(f, "Internal platform error: {msg}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Windows(e) => Some(e),
            _ => None,
        }
    }
}

impl From<windows::core::Error> for Error {
    fn from(e: windows::core::Error) -> Self {
        Self::Windows(e)
    }
}
