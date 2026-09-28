//! Low-Level Keyboard Hook Subsystem (WH_KEYBOARD_LL).
//! Implements Section 2 of WINDOWS.md: sub-millisecond keyboard interception
//! without blocking the Windows message pump.
//!
//! Provides full key mapping for every keyboard key, configurable SR modifier (CapsLock default),
//! and double-tap CapsLock hardware toggle detection.

use bit_sr_core::events::AccessibilityEvent;
use bit_sr_core::input::{
    Key, KeyAction, KeyEvent, KeyModifiers, SRKeyAction, SRKeyConfig, SRModifierTracker,
};
use crossbeam_channel::Sender;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Instant;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    keybd_event, GetAsyncKeyState, GetKeyState, GetKeyboardLayout, GetKeyboardState,
    ToUnicodeEx, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    VK_ADD, VK_APPS, VK_BACK, VK_CAPITAL, VK_CONTROL, VK_DECIMAL, VK_DELETE, VK_DIVIDE,
    VK_DOWN, VK_END, VK_ESCAPE, VK_F1, VK_F24, VK_HOME, VK_INSERT, VK_LCONTROL, VK_LEFT,
    VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MEDIA_NEXT_TRACK, VK_MEDIA_PLAY_PAUSE,
    VK_MEDIA_PREV_TRACK, VK_MEDIA_STOP, VK_MENU, VK_MULTIPLY, VK_NEXT, VK_NUMLOCK,
    VK_OEM_1, VK_OEM_2, VK_OEM_3, VK_OEM_4, VK_OEM_5, VK_OEM_6, VK_OEM_7, VK_OEM_COMMA,
    VK_OEM_MINUS, VK_OEM_PERIOD, VK_OEM_PLUS, VK_PAUSE, VK_PRIOR, VK_RCONTROL, VK_RETURN,
    VK_RIGHT, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SCROLL, VK_SHIFT, VK_SNAPSHOT, VK_SPACE,
    VK_SUBTRACT, VK_TAB, VK_UP, VK_VOLUME_DOWN, VK_VOLUME_MUTE, VK_VOLUME_UP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetForegroundWindow, GetMessageW, GetWindowThreadProcessId,
    PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx,
    KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_QUIT,
};

static HOOK_CHANNEL: Mutex<Option<Sender<AccessibilityEvent>>> = Mutex::new(None);
static IS_RUNNING: AtomicBool = AtomicBool::new(false);
static SR_TRACKER: Mutex<Option<SRModifierTracker>> = Mutex::new(None);
static INPUT_HELP_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Activates or deactivates input help mode in the low-level hook.
pub fn set_input_help_active(active: bool) {
    INPUT_HELP_ACTIVE.store(active, Ordering::SeqCst);
}

/// Checks if input help mode is active.
pub fn is_input_help_active() -> bool {
    INPUT_HELP_ACTIVE.load(Ordering::SeqCst)
}

const LLKHF_EXTENDED: u32 = 0x01;
const LLKHF_INJECTED: u32 = 0x10;
const LLKHF_UP: u32 = 0x80;

/// Maps a Windows Virtual Key code and extended flag to a platform-agnostic Key.
pub fn vk_to_key(vk_code: u32, is_extended: bool) -> Key {
    match vk_code {
        // Letters A - Z (0x41 ..= 0x5A)
        0x41 => Key::A, 0x42 => Key::B, 0x43 => Key::C, 0x44 => Key::D, 0x45 => Key::E,
        0x46 => Key::F, 0x47 => Key::G, 0x48 => Key::H, 0x49 => Key::I, 0x4A => Key::J,
        0x4B => Key::K, 0x4C => Key::L, 0x4D => Key::M, 0x4E => Key::N, 0x4F => Key::O,
        0x50 => Key::P, 0x51 => Key::Q, 0x52 => Key::R, 0x53 => Key::S, 0x54 => Key::T,
        0x55 => Key::U, 0x56 => Key::V, 0x57 => Key::W, 0x58 => Key::X, 0x59 => Key::Y,
        0x5A => Key::Z,

        // Numbers 0 - 9 (0x30 ..= 0x39)
        0x30 => Key::Num0, 0x31 => Key::Num1, 0x32 => Key::Num2, 0x33 => Key::Num3,
        0x34 => Key::Num4, 0x35 => Key::Num5, 0x36 => Key::Num6, 0x37 => Key::Num7,
        0x38 => Key::Num8, 0x39 => Key::Num9,

        // Function keys F1 - F24 (0x70 ..= 0x87)
        c if (VK_F1.0 as u32..=VK_F24.0 as u32).contains(&c) => match c - VK_F1.0 as u32 {
            0 => Key::F1, 1 => Key::F2, 2 => Key::F3, 3 => Key::F4,
            4 => Key::F5, 5 => Key::F6, 6 => Key::F7, 7 => Key::F8,
            8 => Key::F9, 9 => Key::F10, 10 => Key::F11, 11 => Key::F12,
            12 => Key::F13, 13 => Key::F14, 14 => Key::F15, 15 => Key::F16,
            16 => Key::F17, 17 => Key::F18, 18 => Key::F19, 19 => Key::F20,
            20 => Key::F21, 21 => Key::F22, 22 => Key::F23, 23 => Key::F24,
            _ => Key::Other(c),
        },

        // Navigation and Cursor Control
        c if c == VK_ESCAPE.0 as u32 => Key::Escape,
        c if c == VK_RETURN.0 as u32 => {
            if is_extended {
                Key::NumpadEnter
            } else {
                Key::Enter
            }
        }
        c if c == VK_TAB.0 as u32 => Key::Tab,
        c if c == VK_SPACE.0 as u32 => Key::Space,
        c if c == VK_BACK.0 as u32 => Key::Backspace,
        c if c == VK_DELETE.0 as u32 => Key::Delete,
        c if c == VK_INSERT.0 as u32 => Key::Insert,
        c if c == VK_HOME.0 as u32 => Key::Home,
        c if c == VK_END.0 as u32 => Key::End,
        c if c == VK_PRIOR.0 as u32 => Key::PageUp,
        c if c == VK_NEXT.0 as u32 => Key::PageDown,
        c if c == VK_LEFT.0 as u32 => Key::LeftArrow,
        c if c == VK_RIGHT.0 as u32 => Key::RightArrow,
        c if c == VK_UP.0 as u32 => Key::UpArrow,
        c if c == VK_DOWN.0 as u32 => Key::DownArrow,

        // Locks & System
        c if c == VK_CAPITAL.0 as u32 => Key::CapsLock,
        c if c == VK_SCROLL.0 as u32 => Key::ScrollLock,
        c if c == VK_NUMLOCK.0 as u32 => Key::NumLock,
        c if c == VK_SNAPSHOT.0 as u32 => Key::PrintScreen,
        c if c == VK_PAUSE.0 as u32 => Key::Pause,

        // Modifiers
        c if c == VK_LSHIFT.0 as u32 => Key::LeftShift,
        c if c == VK_RSHIFT.0 as u32 => Key::RightShift,
        c if c == VK_SHIFT.0 as u32 => Key::LeftShift,
        c if c == VK_LCONTROL.0 as u32 => Key::LeftControl,
        c if c == VK_RCONTROL.0 as u32 => Key::RightControl,
        c if c == VK_CONTROL.0 as u32 => Key::LeftControl,
        c if c == VK_LMENU.0 as u32 => Key::LeftAlt,
        c if c == VK_RMENU.0 as u32 => Key::RightAlt,
        c if c == VK_MENU.0 as u32 => Key::LeftAlt,
        c if c == VK_LWIN.0 as u32 => Key::LeftSuper,
        c if c == VK_RWIN.0 as u32 => Key::RightSuper,
        c if c == VK_APPS.0 as u32 => Key::Menu,

        // Numpad Keys (0x60 ..= 0x69)
        0x60 => Key::Numpad0, 0x61 => Key::Numpad1, 0x62 => Key::Numpad2,
        0x63 => Key::Numpad3, 0x64 => Key::Numpad4, 0x65 => Key::Numpad5,
        0x66 => Key::Numpad6, 0x67 => Key::Numpad7, 0x68 => Key::Numpad8,
        0x69 => Key::Numpad9,
        c if c == VK_MULTIPLY.0 as u32 => Key::NumpadMultiply,
        c if c == VK_ADD.0 as u32 => Key::NumpadAdd,
        c if c == VK_SUBTRACT.0 as u32 => Key::NumpadSubtract,
        c if c == VK_DECIMAL.0 as u32 => Key::NumpadDecimal,
        c if c == VK_DIVIDE.0 as u32 => Key::NumpadDivide,

        // OEM / Symbols
        c if c == VK_OEM_1.0 as u32 => Key::Semicolon,
        c if c == VK_OEM_PLUS.0 as u32 => Key::Equals,
        c if c == VK_OEM_COMMA.0 as u32 => Key::Comma,
        c if c == VK_OEM_MINUS.0 as u32 => Key::Minus,
        c if c == VK_OEM_PERIOD.0 as u32 => Key::Period,
        c if c == VK_OEM_2.0 as u32 => Key::Slash,
        c if c == VK_OEM_3.0 as u32 => Key::Grave,
        c if c == VK_OEM_4.0 as u32 => Key::LeftBracket,
        c if c == VK_OEM_5.0 as u32 => Key::Backslash,
        c if c == VK_OEM_6.0 as u32 => Key::RightBracket,
        c if c == VK_OEM_7.0 as u32 => Key::Apostrophe,

        // Media Keys
        c if c == VK_VOLUME_MUTE.0 as u32 => Key::VolumeMute,
        c if c == VK_VOLUME_DOWN.0 as u32 => Key::VolumeDown,
        c if c == VK_VOLUME_UP.0 as u32 => Key::VolumeUp,
        c if c == VK_MEDIA_NEXT_TRACK.0 as u32 => Key::MediaNextTrack,
        c if c == VK_MEDIA_PREV_TRACK.0 as u32 => Key::MediaPrevTrack,
        c if c == VK_MEDIA_STOP.0 as u32 => Key::MediaStop,
        c if c == VK_MEDIA_PLAY_PAUSE.0 as u32 => Key::MediaPlayPause,

        other => Key::Other(other),
    }
}

/// Synthetically toggles the physical Windows CapsLock state and LED.
pub unsafe fn toggle_hardware_caps_lock() {
    unsafe {
        keybd_event(VK_CAPITAL.0 as u8, 0x45, KEYBD_EVENT_FLAGS(0), 0);
        keybd_event(VK_CAPITAL.0 as u8, 0x45, KEYEVENTF_KEYUP, 0);
    }
}

/// Queries physical modifier keys from Windows hardware state.
pub fn get_current_modifiers() -> KeyModifiers {
    let mut mods = KeyModifiers::empty();
    unsafe {
        if (GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000) != 0
            || (GetAsyncKeyState(VK_LSHIFT.0 as i32) as u16 & 0x8000) != 0
            || (GetAsyncKeyState(VK_RSHIFT.0 as i32) as u16 & 0x8000) != 0
        {
            mods |= KeyModifiers::SHIFT;
        }
        if (GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0
            || (GetAsyncKeyState(VK_LCONTROL.0 as i32) as u16 & 0x8000) != 0
            || (GetAsyncKeyState(VK_RCONTROL.0 as i32) as u16 & 0x8000) != 0
        {
            mods |= KeyModifiers::CONTROL;
        }
        if (GetAsyncKeyState(VK_MENU.0 as i32) as u16 & 0x8000) != 0
            || (GetAsyncKeyState(VK_LMENU.0 as i32) as u16 & 0x8000) != 0
            || (GetAsyncKeyState(VK_RMENU.0 as i32) as u16 & 0x8000) != 0
        {
            mods |= KeyModifiers::ALT;
        }
        if (GetAsyncKeyState(VK_LWIN.0 as i32) as u16 & 0x8000) != 0
            || (GetAsyncKeyState(VK_RWIN.0 as i32) as u16 & 0x8000) != 0
        {
            mods |= KeyModifiers::SUPER;
        }
    }

    // Check if the SR modifier is active in the tracker
    if let Ok(guard) = SR_TRACKER.lock() {
        if let Some(ref tracker) = *guard {
            if tracker.is_sr_held() {
                mods |= KeyModifiers::SR;
            }
        }
    }

    mods
}

/// Configures the SR modifier keys and double-tap parameters.
pub fn configure_sr_keys(config: SRKeyConfig) {
    if let Ok(mut guard) = SR_TRACKER.lock() {
        *guard = Some(SRModifierTracker::new(config));
    }
}

/// Queries the typed unicode character for a physical key down using ToUnicodeEx.
/// Uses the TM_DONT_MODIFY_KEY_STATE flag (0x04) to avoid destroying dead key / keyboard state.
unsafe fn get_typed_character(vk_code: u32, scan_code: u32) -> Option<String> {
    unsafe {
        let hwnd = GetForegroundWindow();
        let thread_id = if !hwnd.0.is_null() {
            GetWindowThreadProcessId(hwnd, None)
        } else {
            0
        };
        let hkl = GetKeyboardLayout(thread_id);

        let mut key_state = [0u8; 256];
        let _ = GetKeyboardState(&mut key_state);

        let mut char_buf = [0u16; 8];
        // Flag 0x0004 is TM_DONT_MODIFY_KEY_STATE (Windows 10 RS2+)
        let ret = ToUnicodeEx(
            vk_code,
            scan_code,
            &key_state,
            &mut char_buf,
            0x0004,
            Some(hkl),
        );

        if ret > 0 {
            let s = String::from_utf16_lossy(&char_buf[..ret as usize]);
            let trimmed: String = s.chars().filter(|c| !c.is_control()).collect();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }
        None
    }
}

/// The low-level keyboard hook callback procedure.
/// Critical invariant: NEVER execute COM calls or blocking work here.
unsafe extern "system" fn low_level_keyboard_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code >= 0 {
        let kbd = unsafe { *(l_param.0 as *const KBDLLHOOKSTRUCT) };
        let is_up = (kbd.flags.0 & LLKHF_UP) != 0;
        let is_extended = (kbd.flags.0 & LLKHF_EXTENDED) != 0;
        let is_injected = (kbd.flags.0 & LLKHF_INJECTED) != 0;
        let action = if is_up { KeyAction::Up } else { KeyAction::Down };

        // Injected events bypass our SR interception (e.g. when we toggle CapsLock synthetically)
        if is_injected {
            return unsafe { CallNextHookEx(None, n_code, w_param, l_param) };
        }

        let key = vk_to_key(kbd.vkCode, is_extended);
        let mut sr_action = SRKeyAction::None;

        if let Ok(mut guard) = SR_TRACKER.lock() {
            let tracker = guard.get_or_insert_with(|| SRModifierTracker::new(SRKeyConfig::default()));
            let now = Instant::now();
            sr_action = match action {
                KeyAction::Down => tracker.on_key_down(key, now),
                KeyAction::Up => tracker.on_key_up(key, now),
            };
        }

        // Handle CapsLock double-tap toggle
        if sr_action == SRKeyAction::ToggleCapsLock {
            unsafe {
                toggle_hardware_caps_lock();
            }
            let is_on = unsafe { (GetKeyState(VK_CAPITAL.0 as i32) as u16 & 0x0001) != 0 };
            if let Ok(guard) = HOOK_CHANNEL.lock() {
                if let Some(ref tx) = *guard {
                    let _ = tx.try_send(AccessibilityEvent::CapsLockToggled(is_on));
                }
            }
            return LRESULT(1);
        }

        let mut modifiers = get_current_modifiers();
        if sr_action == SRKeyAction::InterceptModifier && key != Key::CapsLock {
            modifiers |= KeyModifiers::SR;
        }

        // Query typed character only on key down when non-modifier and without command modifiers
        let text = if action == KeyAction::Down
            && !key.is_modifier()
            && !modifiers.contains(KeyModifiers::SR)
            && !modifiers.contains(KeyModifiers::ALT)
            && !modifiers.contains(KeyModifiers::CONTROL)
            && !modifiers.contains(KeyModifiers::SUPER)
        {
            unsafe { get_typed_character(kbd.vkCode, kbd.scanCode) }
        } else {
            None
        };

        let key_event = KeyEvent {
            key,
            vk_code: kbd.vkCode,
            scan_code: kbd.scanCode,
            is_extended,
            is_injected,
            action,
            modifiers,
            text,
        };

        if let Ok(guard) = HOOK_CHANNEL.lock() {
            if let Some(ref tx) = *guard {
                // Instantly signal speech interrupt on any key down that is not a pure modifier
                if action == KeyAction::Down && !key.is_modifier() {
                    let _ = tx.try_send(AccessibilityEvent::SpeechInterrupt);
                }
                let _ = tx.try_send(AccessibilityEvent::Input(key_event));
            }
        }

        // Intercept SR modifier from reaching the underlying window
        if sr_action == SRKeyAction::InterceptModifier {
            return LRESULT(1);
        }

        // If input help mode is active, intercept all non-injected keys so they don't affect windows
        if INPUT_HELP_ACTIVE.load(Ordering::Relaxed) {
            return LRESULT(1);
        }
    }

    unsafe { CallNextHookEx(None, n_code, w_param, l_param) }
}

/// Controller handle for the low-level keyboard hook thread.
pub struct KeyboardHookHandle {
    thread_id: u32,
}

impl KeyboardHookHandle {
    /// Launches the low-level keyboard hook on a dedicated thread with a Win32 message pump.
    pub fn start(tx: Sender<AccessibilityEvent>) -> Result<Self, crate::error::Error> {
        let (thread_ready_tx, thread_ready_rx) = crossbeam_channel::bounded(1);

        std::thread::Builder::new()
            .name("bit_sr_keyboard_hook".to_string())
            .spawn(move || unsafe {
                let thread_id = windows::Win32::System::Threading::GetCurrentThreadId();
                if let Ok(mut guard) = HOOK_CHANNEL.lock() {
                    *guard = Some(tx);
                }

                // Initialize SR tracker if not already initialized
                if let Ok(mut guard) = SR_TRACKER.lock() {
                    if guard.is_none() {
                        *guard = Some(SRModifierTracker::new(SRKeyConfig::default()));
                    }
                }

                let hook = match SetWindowsHookExW(
                    WH_KEYBOARD_LL,
                    Some(low_level_keyboard_proc),
                    None,
                    0,
                ) {
                    Ok(h) => h,
                    Err(e) => {
                        let _ = thread_ready_tx.send(Err(crate::error::Error::Windows(e)));
                        return;
                    }
                };

                IS_RUNNING.store(true, Ordering::SeqCst);
                let _ = thread_ready_tx.send(Ok(thread_id));

                let mut msg = MSG::default();
                while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                    // Loop pumps hook callbacks
                }

                let _ = UnhookWindowsHookEx(hook);
                IS_RUNNING.store(false, Ordering::SeqCst);
                if let Ok(mut guard) = HOOK_CHANNEL.lock() {
                    *guard = None;
                }
            })
            .map_err(|e| crate::error::Error::Internal(e.to_string()))?;

        let thread_id = thread_ready_rx
            .recv()
            .map_err(|_| crate::error::Error::HookInstallationFailed("Channel disconnected"))??;

        Ok(Self { thread_id })
    }

    /// Stops the keyboard hook and unregisters it cleanly.
    pub fn stop(self) {
        if IS_RUNNING.load(Ordering::SeqCst) {
            unsafe {
                let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vk_to_key_mappings() {
        assert_eq!(vk_to_key(0x54, false), Key::T);
        assert_eq!(vk_to_key(0x51, false), Key::Q);
        assert_eq!(vk_to_key(0x1B, false), Key::Escape);
        assert_eq!(vk_to_key(0x14, false), Key::CapsLock);
        assert_eq!(vk_to_key(0x2D, false), Key::Insert);
        assert_eq!(vk_to_key(0x25, false), Key::LeftArrow);
        assert_eq!(vk_to_key(0x70, false), Key::F1);
        assert_eq!(vk_to_key(0x7B, false), Key::F12);
        assert_eq!(vk_to_key(0x0D, false), Key::Enter);
        assert_eq!(vk_to_key(0x0D, true), Key::NumpadEnter);
        assert_eq!(vk_to_key(0xBA, false), Key::Semicolon);
    }
}
