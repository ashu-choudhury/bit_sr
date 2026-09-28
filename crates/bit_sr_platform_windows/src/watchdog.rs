//! Deadlock Prevention and Hung Window Detection.
//! Enforces the rule that cross-process calls must never block the screen reader.

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    IsHungAppWindow, SendMessageTimeoutW, SMTO_ABORTIFHUNG, SMTO_NORMAL,
};

/// Returns true if the target window is flagged by Windows as hung/unresponsive.
pub fn is_window_hung(hwnd: HWND) -> bool {
    unsafe { IsHungAppWindow(hwnd).as_bool() }
}

/// Sends a Win32 message with a timeout, aborting immediately if the target process is hung.
pub fn safe_send_message_timeout(
    hwnd: HWND,
    msg: u32,
    w_param: WPARAM,
    l_param: LPARAM,
    timeout_ms: u32,
) -> Option<usize> {
    if is_window_hung(hwnd) {
        log::warn!("Aborting SendMessage to hung window: 0x{:X}", hwnd.0 as usize);
        return None;
    }

    let mut result: usize = 0;
    let flags = SMTO_ABORTIFHUNG | SMTO_NORMAL;
    let res = unsafe {
        SendMessageTimeoutW(
            hwnd,
            msg,
            w_param,
            l_param,
            flags,
            timeout_ms,
            Some(&mut result),
        )
    };

    if res.0 == 0 {
        None
    } else {
        Some(result)
    }
}
