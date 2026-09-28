//! COM Apartment Initialization and RAII Guards.
//! UI Automation requires a dedicated thread in a Multi-Threaded Apartment (MTA).

use crate::error::{Error, Result};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};

/// RAII Guard that uninitializes COM when dropped.
pub struct ComGuard {
    _private: (),
}

impl ComGuard {
    /// Initializes COM on the current thread as Multi-Threaded Apartment (MTA).
    pub fn init_mta() -> Result<Self> {
        unsafe {
            let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
            if hr.is_err() {
                // S_FALSE (0x00000001) means already initialized on this thread, which is fine.
                // RPC_E_CHANGED_MODE means initialized with different apartment (STA), which is an error.
                return Err(Error::Windows(hr.into()));
            }
        }
        Ok(Self { _private: () })
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}
