//! Multi-Desktop and Windows Secure Desktop (UAC / Winlogon) Tracking.

use windows::Win32::System::StationsAndDesktops::{
    CloseDesktop, OpenInputDesktop, DESKTOP_CONTROL_FLAGS, DESKTOP_SWITCHDESKTOP,
};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW};

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

/// Retrieves the title of the current active foreground top-level window.
pub fn get_foreground_window_title() -> Option<String> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }

        let mut buf = [0u16; 512];
        let len = GetWindowTextW(hwnd, &mut buf);
        if len > 0 {
            let title = String::from_utf16_lossy(&buf[..len as usize]);
            let trimmed = title.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
        None
    }
}

/// Queries the Windows operating system default user UI locale name (e.g., "en-US", "es-ES", "hi-IN").
pub fn get_user_default_locale_name() -> Option<String> {
    unsafe {
        let mut buffer = [0u16; 85];
        let len = windows::Win32::Globalization::GetUserDefaultLocaleName(&mut buffer);
        if len > 1 {
            let name = String::from_utf16_lossy(&buffer[..(len - 1) as usize]);
            let trimmed = name.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_user_default_locale_name() {
        let locale = get_user_default_locale_name();
        assert!(locale.is_some(), "Expected Windows to return a user default locale name");
        let loc_str = locale.unwrap();
        assert!(!loc_str.is_empty());
        println!("Detected Windows default locale: {}", loc_str);
    }
}

