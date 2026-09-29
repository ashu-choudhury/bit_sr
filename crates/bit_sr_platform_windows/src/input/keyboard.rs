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
use crossbeam_channel::{Receiver, Sender};
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::time::Instant;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    keybd_event, GetAsyncKeyState, GetKeyState, GetKeyboardLayout,
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

static IS_RUNNING: AtomicBool = AtomicBool::new(false);
static INPUT_HELP_ACTIVE: AtomicBool = AtomicBool::new(false);
static CURRENT_MODIFIERS: AtomicU32 = AtomicU32::new(0);

static CONFIG_USE_CAPSLOCK: AtomicBool = AtomicBool::new(true);
static CONFIG_USE_INSERT: AtomicBool = AtomicBool::new(false);
static CONFIG_USE_NUMPAD_INSERT: AtomicBool = AtomicBool::new(false);
static CONFIG_DOUBLE_TAP_MS: AtomicU64 = AtomicU64::new(350);

#[derive(Debug, Clone, Copy)]
struct RawKeyboardEvent {
    key: Key,
    vk_code: u32,
    scan_code: u32,
    is_extended: bool,
    is_injected: bool,
    action: KeyAction,
    modifiers: KeyModifiers,
}

struct HookThreadState {
    tx: Sender<AccessibilityEvent>,
    raw_tx: Sender<RawKeyboardEvent>,
    sr_tracker: SRModifierTracker,
    shift_left: bool,
    shift_right: bool,
    ctrl_left: bool,
    ctrl_right: bool,
    alt_left: bool,
    alt_right: bool,
    super_left: bool,
    super_right: bool,
}

thread_local! {
    static HOOK_STATE: RefCell<Option<HookThreadState>> = const { RefCell::new(None) };
}

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

/// Queries currently active modifier keys without making Win32 API calls.
/// Reads atomic bitflags updated directly by the low-level keyboard hook.
pub fn get_current_modifiers() -> KeyModifiers {
    KeyModifiers::from_bits_truncate(CURRENT_MODIFIERS.load(Ordering::Relaxed))
}

/// Configures the SR modifier keys and double-tap parameters.
pub fn configure_sr_keys(config: SRKeyConfig) {
    CONFIG_USE_CAPSLOCK.store(config.use_caps_lock, Ordering::Release);
    CONFIG_USE_INSERT.store(config.use_insert, Ordering::Release);
    CONFIG_USE_NUMPAD_INSERT.store(config.use_numpad_insert, Ordering::Release);
    CONFIG_DOUBLE_TAP_MS.store(config.double_tap_timeout_ms, Ordering::Release);
}

/// Queries the typed unicode character for a physical key down using ToUnicodeEx.
/// Runs on the dedicated keyboard worker thread outside the Windows hook callback.
/// Constructs the 256-byte key state directly from tracked modifier bits rather than
/// querying the OS 256 times per keystroke.
unsafe fn decode_character(
    vk_code: u32,
    scan_code: u32,
    modifiers: KeyModifiers,
    key: Key,
) -> Option<String> {
    unsafe {
        let hwnd = GetForegroundWindow();
        let thread_id = if !hwnd.0.is_null() {
            GetWindowThreadProcessId(hwnd, None)
        } else {
            0
        };
        let hkl = GetKeyboardLayout(thread_id);

        let is_caps = (GetKeyState(VK_CAPITAL.0 as i32) as u16 & 0x0001) != 0;
        let is_num = (GetKeyState(VK_NUMLOCK.0 as i32) as u16 & 0x0001) != 0;

        // Build key state directly with only the 5 required flags that ToUnicodeEx inspects
        let mut key_state = [0u8; 256];
        if modifiers.contains(KeyModifiers::SHIFT) {
            key_state[VK_SHIFT.0 as usize] = 0x80;
        }
        if modifiers.contains(KeyModifiers::CONTROL) {
            key_state[VK_CONTROL.0 as usize] = 0x80;
        }
        if modifiers.contains(KeyModifiers::ALT) {
            key_state[VK_MENU.0 as usize] = 0x80;
        }
        if is_caps {
            key_state[VK_CAPITAL.0 as usize] = 0x01;
        }
        if is_num {
            key_state[VK_NUMLOCK.0 as usize] = 0x01;
        }

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

        // Direct fallback: only for letters and digits if ToUnicodeEx returned nothing.
        // We do not hardcode symbols to avoid speaking wrong characters on international layouts (e.g. AZERTY / QWERTZ).
        let is_upper = modifiers.contains(KeyModifiers::SHIFT) ^ is_caps;
        match key {
            Key::A => Some(if is_upper { "A" } else { "a" }.to_string()),
            Key::B => Some(if is_upper { "B" } else { "b" }.to_string()),
            Key::C => Some(if is_upper { "C" } else { "c" }.to_string()),
            Key::D => Some(if is_upper { "D" } else { "d" }.to_string()),
            Key::E => Some(if is_upper { "E" } else { "e" }.to_string()),
            Key::F => Some(if is_upper { "F" } else { "f" }.to_string()),
            Key::G => Some(if is_upper { "G" } else { "g" }.to_string()),
            Key::H => Some(if is_upper { "H" } else { "h" }.to_string()),
            Key::I => Some(if is_upper { "I" } else { "i" }.to_string()),
            Key::J => Some(if is_upper { "J" } else { "j" }.to_string()),
            Key::K => Some(if is_upper { "K" } else { "k" }.to_string()),
            Key::L => Some(if is_upper { "L" } else { "l" }.to_string()),
            Key::M => Some(if is_upper { "M" } else { "m" }.to_string()),
            Key::N => Some(if is_upper { "N" } else { "n" }.to_string()),
            Key::O => Some(if is_upper { "O" } else { "o" }.to_string()),
            Key::P => Some(if is_upper { "P" } else { "p" }.to_string()),
            Key::Q => Some(if is_upper { "Q" } else { "q" }.to_string()),
            Key::R => Some(if is_upper { "R" } else { "r" }.to_string()),
            Key::S => Some(if is_upper { "S" } else { "s" }.to_string()),
            Key::T => Some(if is_upper { "T" } else { "t" }.to_string()),
            Key::U => Some(if is_upper { "U" } else { "u" }.to_string()),
            Key::V => Some(if is_upper { "V" } else { "v" }.to_string()),
            Key::W => Some(if is_upper { "W" } else { "w" }.to_string()),
            Key::X => Some(if is_upper { "X" } else { "x" }.to_string()),
            Key::Y => Some(if is_upper { "Y" } else { "y" }.to_string()),
            Key::Z => Some(if is_upper { "Z" } else { "z" }.to_string()),

            Key::Num0 => Some("0".to_string()),
            Key::Num1 => Some("1".to_string()),
            Key::Num2 => Some("2".to_string()),
            Key::Num3 => Some("3".to_string()),
            Key::Num4 => Some("4".to_string()),
            Key::Num5 => Some("5".to_string()),
            Key::Num6 => Some("6".to_string()),
            Key::Num7 => Some("7".to_string()),
            Key::Num8 => Some("8".to_string()),
            Key::Num9 => Some("9".to_string()),

            Key::Space => Some(" ".to_string()),

            _ => None,
        }
    }
}

/// Spawns the dedicated keyboard worker thread that handles character decoding off the hook callback.
fn spawn_keyboard_worker(
    raw_rx: Receiver<RawKeyboardEvent>,
    tx: Sender<AccessibilityEvent>,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("bit_sr_keyboard_worker".to_string())
        .spawn(move || {
            while let Ok(raw) = raw_rx.recv() {
                // Character decoding only needed for key down on non-modifiers without command modifiers
                let text = if raw.action == KeyAction::Down
                    && !raw.key.is_modifier()
                    && !raw.modifiers.intersects(
                        KeyModifiers::SR | KeyModifiers::ALT | KeyModifiers::CONTROL | KeyModifiers::SUPER,
                    )
                {
                    unsafe { decode_character(raw.vk_code, raw.scan_code, raw.modifiers, raw.key) }
                } else {
                    None
                };

                let key_event = KeyEvent {
                    key: raw.key,
                    vk_code: raw.vk_code,
                    scan_code: raw.scan_code,
                    is_extended: raw.is_extended,
                    is_injected: raw.is_injected,
                    action: raw.action,
                    modifiers: raw.modifiers,
                    text,
                };

                let _ = tx.try_send(AccessibilityEvent::Input(key_event));
            }
        })
        .expect("Failed to spawn keyboard worker thread")
}

/// The low-level keyboard hook callback procedure.
/// Critical invariant: NEVER execute COM calls, blocking locks, or heavy decoding here.
/// Minimal execution path: inspect event, update local state, try_send to queue, and return immediately.
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

        // Injected events bypass our SR interception (e.g. synthetic CapsLock toggle)
        if is_injected {
            return unsafe { CallNextHookEx(None, n_code, w_param, l_param) };
        }

        let key = vk_to_key(kbd.vkCode, is_extended);

        let intercept = HOOK_STATE.with(|cell| {
            let mut guard = match cell.try_borrow_mut() {
                Ok(g) => g,
                Err(_) => return false,
            };
            let state = match guard.as_mut() {
                Some(s) => s,
                None => return false,
            };

            // Synchronize SR tracker config if changed
            state.sr_tracker.config.use_caps_lock = CONFIG_USE_CAPSLOCK.load(Ordering::Relaxed);
            state.sr_tracker.config.use_insert = CONFIG_USE_INSERT.load(Ordering::Relaxed);
            state.sr_tracker.config.use_numpad_insert = CONFIG_USE_NUMPAD_INSERT.load(Ordering::Relaxed);
            state.sr_tracker.config.double_tap_timeout_ms = CONFIG_DOUBLE_TAP_MS.load(Ordering::Relaxed);

            // Update physical modifier tracking
            let is_down = action == KeyAction::Down;
            match key {
                Key::LeftShift => state.shift_left = is_down,
                Key::RightShift => state.shift_right = is_down,
                Key::LeftControl => state.ctrl_left = is_down,
                Key::RightControl => state.ctrl_right = is_down,
                Key::LeftAlt => state.alt_left = is_down,
                Key::RightAlt => state.alt_right = is_down,
                Key::LeftSuper => state.super_left = is_down,
                Key::RightSuper => state.super_right = is_down,
                _ => {}
            }

            let now = Instant::now();
            let sr_action = match action {
                KeyAction::Down => state.sr_tracker.on_key_down(key, now),
                KeyAction::Up => state.sr_tracker.on_key_up(key, now),
            };

            // Handle CapsLock double-tap toggle
            if sr_action == SRKeyAction::ToggleCapsLock {
                unsafe {
                    toggle_hardware_caps_lock();
                }
                let is_on = unsafe { (GetKeyState(VK_CAPITAL.0 as i32) as u16 & 0x0001) != 0 };
                let _ = state.tx.try_send(AccessibilityEvent::CapsLockToggled(is_on));
                return true;
            }

            // Build active modifiers bitflag
            let mut modifiers = KeyModifiers::empty();
            if state.shift_left || state.shift_right {
                modifiers |= KeyModifiers::SHIFT;
            }
            if state.ctrl_left || state.ctrl_right {
                modifiers |= KeyModifiers::CONTROL;
            }
            if state.alt_left || state.alt_right {
                modifiers |= KeyModifiers::ALT;
            }
            if state.super_left || state.super_right {
                modifiers |= KeyModifiers::SUPER;
            }
            if state.sr_tracker.is_sr_held() || (sr_action == SRKeyAction::InterceptModifier && key != Key::CapsLock) {
                modifiers |= KeyModifiers::SR;
            }

            CURRENT_MODIFIERS.store(modifiers.bits(), Ordering::Relaxed);

            // Instantly signal speech interrupt on physical Control or Escape press
            if action == KeyAction::Down
                && (key == Key::LeftControl || key == Key::RightControl || key == Key::Escape)
            {
                let _ = state.tx.try_send(AccessibilityEvent::SpeechInterrupt);
            }

            // Dispatch raw event to worker thread queue for non-blocking Unicode translation
            let raw_event = RawKeyboardEvent {
                key,
                vk_code: kbd.vkCode,
                scan_code: kbd.scanCode,
                is_extended,
                is_injected,
                action,
                modifiers,
            };
            let _ = state.raw_tx.try_send(raw_event);

            // Intercept SR modifier from reaching the underlying window
            if sr_action == SRKeyAction::InterceptModifier {
                return true;
            }

            // Intercept all keys in input help mode
            if INPUT_HELP_ACTIVE.load(Ordering::Relaxed) {
                return true;
            }

            false
        });

        if intercept {
            return LRESULT(1);
        }
    }

    unsafe { CallNextHookEx(None, n_code, w_param, l_param) }
}

/// Controller handle for the low-level keyboard hook thread.
pub struct KeyboardHookHandle {
    hook_thread_id: u32,
    _worker_thread: std::thread::JoinHandle<()>,
}

impl KeyboardHookHandle {
    /// Launches the low-level keyboard hook on a dedicated thread with a Win32 message pump,
    /// and spawns a background worker thread for non-blocking Unicode character decoding.
    pub fn start(tx: Sender<AccessibilityEvent>) -> Result<Self, crate::error::Error> {
        let (thread_ready_tx, thread_ready_rx) = crossbeam_channel::bounded(1);
        let (raw_tx, raw_rx) = crossbeam_channel::bounded::<RawKeyboardEvent>(256);

        // Spawn dedicated worker thread for non-blocking character decoding
        let worker_thread = spawn_keyboard_worker(raw_rx, tx.clone());

        std::thread::Builder::new()
            .name("bit_sr_keyboard_hook".to_string())
            .spawn(move || unsafe {
                let thread_id = windows::Win32::System::Threading::GetCurrentThreadId();

                // Initialize modifier keys on startup
                let shift_left = (GetAsyncKeyState(VK_LSHIFT.0 as i32) as u16 & 0x8000) != 0;
                let shift_right = (GetAsyncKeyState(VK_RSHIFT.0 as i32) as u16 & 0x8000) != 0;
                let ctrl_left = (GetAsyncKeyState(VK_LCONTROL.0 as i32) as u16 & 0x8000) != 0;
                let ctrl_right = (GetAsyncKeyState(VK_RCONTROL.0 as i32) as u16 & 0x8000) != 0;
                let alt_left = (GetAsyncKeyState(VK_LMENU.0 as i32) as u16 & 0x8000) != 0;
                let alt_right = (GetAsyncKeyState(VK_RMENU.0 as i32) as u16 & 0x8000) != 0;
                let super_left = (GetAsyncKeyState(VK_LWIN.0 as i32) as u16 & 0x8000) != 0;
                let super_right = (GetAsyncKeyState(VK_RWIN.0 as i32) as u16 & 0x8000) != 0;

                let mut initial_mods = KeyModifiers::empty();
                if shift_left || shift_right { initial_mods |= KeyModifiers::SHIFT; }
                if ctrl_left || ctrl_right { initial_mods |= KeyModifiers::CONTROL; }
                if alt_left || alt_right { initial_mods |= KeyModifiers::ALT; }
                if super_left || super_right { initial_mods |= KeyModifiers::SUPER; }
                CURRENT_MODIFIERS.store(initial_mods.bits(), Ordering::Relaxed);

                let config = SRKeyConfig {
                    use_caps_lock: CONFIG_USE_CAPSLOCK.load(Ordering::Relaxed),
                    use_insert: CONFIG_USE_INSERT.load(Ordering::Relaxed),
                    use_numpad_insert: CONFIG_USE_NUMPAD_INSERT.load(Ordering::Relaxed),
                    double_tap_timeout_ms: CONFIG_DOUBLE_TAP_MS.load(Ordering::Relaxed),
                };

                HOOK_STATE.with(|cell| {
                    *cell.borrow_mut() = Some(HookThreadState {
                        tx,
                        raw_tx,
                        sr_tracker: SRModifierTracker::new(config),
                        shift_left,
                        shift_right,
                        ctrl_left,
                        ctrl_right,
                        alt_left,
                        alt_right,
                        super_left,
                        super_right,
                    });
                });

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

                // Dropping HOOK_STATE drops raw_tx, signaling worker thread to exit cleanly
                HOOK_STATE.with(|cell| {
                    *cell.borrow_mut() = None;
                });
            })
            .map_err(|e| crate::error::Error::Internal(e.to_string()))?;

        let hook_thread_id = thread_ready_rx
            .recv()
            .map_err(|_| crate::error::Error::HookInstallationFailed("Channel disconnected"))??;

        Ok(Self {
            hook_thread_id,
            _worker_thread: worker_thread,
        })
    }

    /// Stops the keyboard hook and unregisters it cleanly.
    pub fn stop(self) {
        if IS_RUNNING.load(Ordering::SeqCst) {
            unsafe {
                let _ = PostThreadMessageW(self.hook_thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
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
