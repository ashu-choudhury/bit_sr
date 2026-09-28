//! Low-Level Keyboard Hook Subsystem (WH_KEYBOARD_LL).
//! Implements Section 2 of WINDOWS.md: sub-millisecond keyboard interception
//! without blocking the Windows message pump.

use bit_sr_core::events::AccessibilityEvent;
use bit_sr_core::input::{KeyAction, KeyEvent, KeyModifiers};
use crossbeam_channel::Sender;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyState, VK_CAPITAL, VK_CONTROL, VK_INSERT, VK_LCONTROL, VK_LMENU,
    VK_LSHIFT, VK_LWIN, VK_MENU, VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx,
    KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_QUIT,
};

static HOOK_CHANNEL: Mutex<Option<Sender<AccessibilityEvent>>> = Mutex::new(None);
static IS_RUNNING: AtomicBool = AtomicBool::new(false);

const LLKHF_EXTENDED: u32 = 0x01;
const LLKHF_INJECTED: u32 = 0x10;
const LLKHF_UP: u32 = 0x80;

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
        if (GetKeyState(VK_CAPITAL.0 as i32) as u16 & 0x0001) != 0 {
            mods |= KeyModifiers::CAPSLOCK;
        }
        if (GetAsyncKeyState(VK_INSERT.0 as i32) as u16 & 0x8000) != 0 {
            mods |= KeyModifiers::INSERT;
        }
    }
    mods
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

        let modifiers = get_current_modifiers();
        let key_event = KeyEvent {
            vk_code: kbd.vkCode,
            scan_code: kbd.scanCode,
            is_extended,
            is_injected,
            action,
            modifiers,
            text: None,
        };

        if let Ok(guard) = HOOK_CHANNEL.lock() {
            if let Some(ref tx) = *guard {
                // Instantly signal speech interrupt on any key down that is not a pure modifier
                if action == KeyAction::Down {
                    let is_pure_modifier = matches!(
                        kbd.vkCode,
                        0x10 | 0x11 | 0x12 | 0x5B | 0x5C | 0xA0 | 0xA1 | 0xA2 | 0xA3 | 0xA4 | 0xA5
                    );
                    if !is_pure_modifier {
                        let _ = tx.try_send(AccessibilityEvent::SpeechInterrupt);
                    }
                }
                let _ = tx.try_send(AccessibilityEvent::Input(key_event));
            }
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
