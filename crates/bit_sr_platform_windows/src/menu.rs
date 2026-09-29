//! Native Windows Screen Reader Popup Menu Implementation (`TrackPopupMenuEx`).
//!
//! Creates an authentic Win32 `#32768` popup menu docked to the left side of the screen.
//! Follows NVDA's clean-room architecture (`wx.Menu.PopupMenu` equivalent) so that:
//! - No separate application window is launched.
//! - Operating system handles menu loop and accessibility natively.
//! - Fully grabs keyboard focus via `AttachThreadInput` and `SetForegroundWindow`.
//! - Automatically focuses the first menu item so screen readers announce it instantly.

use bit_sr_core::events::AccessibilityEvent;
use bit_sr_core::menu::MenuAction;
use crossbeam_channel::Sender;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;
use std::thread;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::UpdateWindow;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    keybd_event, SetActiveWindow, SetFocus, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, VK_DOWN, VK_MENU,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, BringWindowToTop, CreatePopupMenu, CreateWindowExW, DefWindowProcW,
    DestroyMenu, DestroyWindow, GetForegroundWindow, GetSystemMetrics,
    GetWindowThreadProcessId, KillTimer, PostMessageW, RegisterClassExW,
    SetForegroundWindow, SetTimer, ShowWindow, TrackPopupMenuEx,
    CS_HREDRAW, CS_VREDRAW,
    MF_POPUP, MF_SEPARATOR, MF_STRING, SM_CYSCREEN, SW_SHOW,
    TPM_LEFTALIGN, TPM_RETURNCMD, TPM_TOPALIGN,
    WNDCLASSEXW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
    WM_NULL,
};

pub const ID_PREF_SETTINGS: u32 = 101;
pub const ID_PREF_SPEECH: u32 = 102;
pub const ID_PREF_KEYBOARD: u32 = 103;

pub const ID_TOOLS_PLUGINS: u32 = 201;
pub const ID_TOOLS_LOG: u32 = 202;
pub const ID_TOOLS_RELOAD: u32 = 203;

pub const ID_SPEECH_TALK: u32 = 301;
pub const ID_SPEECH_MUTE: u32 = 302;

pub const ID_HELP_INPUT_HELP: u32 = 401;
pub const ID_HELP_ABOUT: u32 = 402;

pub const ID_RESTART: u32 = 501;
pub const ID_EXIT: u32 = 502;

static IS_MENU_ACTIVE: AtomicBool = AtomicBool::new(false);
static REGISTER_CLASS_ONCE: Once = Once::new();
const CLASS_NAME: PCWSTR = w!("bit_sr_menu_host");

unsafe extern "system" fn menu_host_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

fn ensure_window_class_registered() {
    REGISTER_CLASS_ONCE.call_once(|| unsafe {
        let h_instance = GetModuleHandleW(None).unwrap_or_default();
        let wnd_class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(menu_host_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: h_instance.into(),
            hIcon: Default::default(),
            hCursor: Default::default(),
            hbrBackground: Default::default(),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: CLASS_NAME,
            hIconSm: Default::default(),
        };
        let _ = RegisterClassExW(&wnd_class);
    });
}

/// Timer callback that fires once immediately after `TrackPopupMenuEx` starts its modal loop.
/// Synthesizes a `VK_DOWN` keystroke to highlight and focus the first menu item ("Preferences"),
/// causing Windows accessibility to fire `EVENT_OBJECT_FOCUS` so screen readers speak it instantly.
unsafe extern "system" fn auto_select_first_item_timer(
    hwnd: HWND,
    _msg: u32,
    id_event: usize,
    _time: u32,
) {
    if id_event == 42 {
        unsafe {
            let _ = KillTimer(Some(hwnd), 42);
            keybd_event(VK_DOWN.0 as u8, 0, KEYBD_EVENT_FLAGS(0), 0);
            keybd_event(VK_DOWN.0 as u8, 0, KEYEVENTF_KEYUP, 0);
        }
    }
}

fn force_foreground_and_focus(hwnd: HWND) -> HWND {
    unsafe {
        let fg_hwnd = GetForegroundWindow();
        let fg_thread = GetWindowThreadProcessId(fg_hwnd, None);
        let cur_thread = GetCurrentThreadId();

        // 1. Break Windows foreground lock timeout by simulating a harmless Alt key press/release
        keybd_event(VK_MENU.0 as u8, 0, KEYBD_EVENT_FLAGS(0), 0);
        keybd_event(VK_MENU.0 as u8, 0, KEYEVENTF_KEYUP, 0);

        // 2. Attach thread input to the current foreground thread if different
        if fg_thread != 0 && fg_thread != cur_thread {
            let _ = AttachThreadInput(cur_thread, fg_thread, true);
            let _ = BringWindowToTop(hwnd);
            let _ = SetForegroundWindow(hwnd);
            let _ = SetActiveWindow(hwnd);
            let _ = SetFocus(Some(hwnd));
            let _ = AttachThreadInput(cur_thread, fg_thread, false);
        } else {
            let _ = BringWindowToTop(hwnd);
            let _ = SetForegroundWindow(hwnd);
            let _ = SetActiveWindow(hwnd);
            let _ = SetFocus(Some(hwnd));
        }

        fg_hwnd
    }
}

/// Opens the native Windows screen reader popup menu asynchronously.
///
/// Prevents multiple concurrent menu instances and sends the chosen `MenuAction`
/// through an `AccessibilityEvent::MenuAction` back to the engine event loop.
pub fn open_menu_async(event_tx: Sender<AccessibilityEvent>) {
    if IS_MENU_ACTIVE.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        // Menu is already open; do not spawn a duplicate
        return;
    }

    thread::Builder::new()
        .name("bit_sr_win32_menu".to_string())
        .spawn(move || {
            let action = show_native_popup_menu(24, -1);
            IS_MENU_ACTIVE.store(false, Ordering::SeqCst);
            if let Some(act) = action {
                let _ = event_tx.send(AccessibilityEvent::MenuAction(act));
            }
        })
        .ok();
}

/// Displays an authentic native Windows popup menu (identical to NVDA's wx.Menu.PopupMenu)
/// docked to the left side of the screen.
pub fn show_native_popup_menu(x: i32, y: i32) -> Option<MenuAction> {
    ensure_window_class_registered();

    unsafe {
        // 1. Preferences Submenu
        let hmenu_pref = match CreatePopupMenu() {
            Ok(h) => h,
            Err(_) => return None,
        };
        let _ = AppendMenuW(hmenu_pref, MF_STRING, ID_PREF_SETTINGS as usize, w!("&Settings...\tSR+Ctrl+S"));
        let _ = AppendMenuW(hmenu_pref, MF_STRING, ID_PREF_SPEECH as usize, w!("Sp&eech & Voices..."));
        let _ = AppendMenuW(hmenu_pref, MF_STRING, ID_PREF_KEYBOARD as usize, w!("&Keyboard Shortcuts..."));

        // 2. Tools Submenu
        let hmenu_tools = match CreatePopupMenu() {
            Ok(h) => h,
            Err(_) => {
                let _ = DestroyMenu(hmenu_pref);
                return None;
            }
        };
        let _ = AppendMenuW(hmenu_tools, MF_STRING, ID_TOOLS_PLUGINS as usize, w!("&Extension & Plugin Manager..."));
        let _ = AppendMenuW(hmenu_tools, MF_STRING, ID_TOOLS_LOG as usize, w!("&View Diagnostic Log"));
        let _ = AppendMenuW(hmenu_tools, MF_STRING, ID_TOOLS_RELOAD as usize, w!("&Reload Plugins"));

        // 3. Speech Mode Submenu
        let hmenu_speech = match CreatePopupMenu() {
            Ok(h) => h,
            Err(_) => {
                let _ = DestroyMenu(hmenu_pref);
                let _ = DestroyMenu(hmenu_tools);
                return None;
            }
        };
        let _ = AppendMenuW(hmenu_speech, MF_STRING, ID_SPEECH_TALK as usize, w!("&Talk Mode\tSR+S"));
        let _ = AppendMenuW(hmenu_speech, MF_STRING, ID_SPEECH_MUTE as usize, w!("&Mute Mode\tSR+S"));

        // 4. Help Submenu
        let hmenu_help = match CreatePopupMenu() {
            Ok(h) => h,
            Err(_) => {
                let _ = DestroyMenu(hmenu_pref);
                let _ = DestroyMenu(hmenu_tools);
                let _ = DestroyMenu(hmenu_speech);
                return None;
            }
        };
        let _ = AppendMenuW(hmenu_help, MF_STRING, ID_HELP_INPUT_HELP as usize, w!("&Input Help Mode\tSR+1"));
        let _ = AppendMenuW(hmenu_help, MF_SEPARATOR, 0, None);
        let _ = AppendMenuW(hmenu_help, MF_STRING, ID_HELP_ABOUT as usize, w!("&About bit_sr..."));

        // 5. Root Menu
        let hmenu_root = match CreatePopupMenu() {
            Ok(h) => h,
            Err(_) => {
                let _ = DestroyMenu(hmenu_pref);
                let _ = DestroyMenu(hmenu_tools);
                let _ = DestroyMenu(hmenu_speech);
                let _ = DestroyMenu(hmenu_help);
                return None;
            }
        };
        let _ = AppendMenuW(hmenu_root, MF_POPUP, hmenu_pref.0 as usize, w!("&Preferences"));
        let _ = AppendMenuW(hmenu_root, MF_POPUP, hmenu_tools.0 as usize, w!("&Tools"));
        let _ = AppendMenuW(hmenu_root, MF_POPUP, hmenu_speech.0 as usize, w!("&Speech Mode"));
        let _ = AppendMenuW(hmenu_root, MF_POPUP, hmenu_help.0 as usize, w!("&Help"));
        let _ = AppendMenuW(hmenu_root, MF_SEPARATOR, 0, None);
        let _ = AppendMenuW(hmenu_root, MF_STRING, ID_RESTART as usize, w!("&Restart bit_sr"));
        let _ = AppendMenuW(hmenu_root, MF_STRING, ID_EXIT as usize, w!("E&xit bit_sr\tSR+Q"));

        // Position on the left side of the screen
        let chosen_y = if y < 0 {
            let screen_h = GetSystemMetrics(SM_CYSCREEN);
            (screen_h - 220) / 2
        } else {
            y
        };

        // 6. Create dedicated Anchor Window (WS_POPUP | WS_VISIBLE with WS_EX_TOOLWINDOW | WS_EX_TOPMOST)
        let h_instance = GetModuleHandleW(None).unwrap_or_default();
        let hwnd = match CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            CLASS_NAME,
            w!("bit_sr_menu"),
            WS_POPUP | WS_VISIBLE,
            x,
            chosen_y,
            1,
            1,
            None,
            None,
            Some(h_instance.into()),
            None,
        ) {
            Ok(h) => h,
            Err(_) => {
                let _ = DestroyMenu(hmenu_root);
                return None;
            }
        };

        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = UpdateWindow(hwnd);

        // 7. Force Foreground & Keyboard Focus
        let prev_fg = force_foreground_and_focus(hwnd);

        // 8. Arm single-shot timer to auto-focus the first item once modal loop begins
        let _ = SetTimer(Some(hwnd), 42, 15, Some(auto_select_first_item_timer));

        // 9. Track popup menu with native OS modal menu loop
        let selected_cmd = TrackPopupMenuEx(
            hmenu_root,
            (TPM_LEFTALIGN | TPM_TOPALIGN | TPM_RETURNCMD).0,
            x,
            chosen_y,
            hwnd,
            None,
        );

        // 10. Cleanup
        let _ = KillTimer(Some(hwnd), 42);
        let _ = DestroyMenu(hmenu_root);
        let _ = DestroyWindow(hwnd);

        // Standard Win32 context menu cleanup (KB Q135788)
        let _ = PostMessageW(None, WM_NULL, WPARAM(0), LPARAM(0));

        // Restore focus to previous application window
        if prev_fg != HWND::default() && prev_fg != hwnd {
            let _ = SetForegroundWindow(prev_fg);
        }

        map_menu_cmd_to_action(selected_cmd.0 as u32)
    }
}

/// Maps native Win32 popup menu command IDs to platform-agnostic `MenuAction`.
pub fn map_menu_cmd_to_action(cmd_id: u32) -> Option<MenuAction> {
    match cmd_id {
        ID_PREF_SETTINGS => Some(MenuAction::OpenSettings),
        ID_PREF_SPEECH => Some(MenuAction::OpenSpeechSettings),
        ID_PREF_KEYBOARD => Some(MenuAction::OpenKeyboardSettings),
        ID_TOOLS_PLUGINS => Some(MenuAction::OpenPluginManager),
        ID_TOOLS_LOG => Some(MenuAction::ViewLog),
        ID_TOOLS_RELOAD => Some(MenuAction::ReloadPlugins),
        ID_SPEECH_TALK => Some(MenuAction::SetSpeechModeTalk),
        ID_SPEECH_MUTE => Some(MenuAction::SetSpeechModeMute),
        ID_HELP_INPUT_HELP => Some(MenuAction::ToggleInputHelp),
        ID_HELP_ABOUT => Some(MenuAction::About),
        ID_RESTART => Some(MenuAction::Restart),
        ID_EXIT => Some(MenuAction::Quit),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_menu_cmd_mapping() {
        assert_eq!(map_menu_cmd_to_action(ID_PREF_SETTINGS), Some(MenuAction::OpenSettings));
        assert_eq!(map_menu_cmd_to_action(ID_PREF_SPEECH), Some(MenuAction::OpenSpeechSettings));
        assert_eq!(map_menu_cmd_to_action(ID_PREF_KEYBOARD), Some(MenuAction::OpenKeyboardSettings));
        assert_eq!(map_menu_cmd_to_action(ID_TOOLS_PLUGINS), Some(MenuAction::OpenPluginManager));
        assert_eq!(map_menu_cmd_to_action(ID_SPEECH_TALK), Some(MenuAction::SetSpeechModeTalk));
        assert_eq!(map_menu_cmd_to_action(ID_SPEECH_MUTE), Some(MenuAction::SetSpeechModeMute));
        assert_eq!(map_menu_cmd_to_action(ID_EXIT), Some(MenuAction::Quit));
        assert_eq!(map_menu_cmd_to_action(99999), None);
    }
}
