//! Multi-Desktop and Windows Secure Desktop (UAC / Winlogon) Tracking.

use windows::Win32::System::StationsAndDesktops::{
    CloseDesktop, OpenInputDesktop, DESKTOP_CONTROL_FLAGS, DESKTOP_SWITCHDESKTOP,
};

/// Returns true if the system has switched to a Secure Desktop (e.g. UAC prompt, lock screen).
/// When active, a non-elevated user application cannot access input or screen elements.
pub fn is_secure_desktop_active() -> bool {
    unsafe {
        match OpenInputDesktop(DESKTOP_CONTROL_FLAGS(0), false, DESKTOP_SWITCHDESKTOP) {
            Ok(h_desktop) => {
                let _ = CloseDesktop(h_desktop);
                false
            }
            Err(_) => true,
        }
    }
}
