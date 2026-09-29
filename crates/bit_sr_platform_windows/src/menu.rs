//! Native Windows Screen Reader Popup Menu Implementation (`TrackPopupMenuEx`).
//!
//! Creates an authentic Win32 `#32768` popup menu docked to the left side of the screen.
//! Follows NVDA's clean-room architecture (`wx.Menu.PopupMenu` equivalent) so that:
//! - No separate application window is launched.
//! - Operating system handles menu loop and accessibility natively.
//! - Accessible via arrows, Enter, Esc, and keyboard accelerators.

use bit_sr_core::events::AccessibilityEvent;
use bit_sr_core::menu::MenuAction;
use crossbeam_channel::Sender;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

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
    unsafe {
        use windows::core::w;
        use windows::Win32::UI::WindowsAndMessaging::{
            AppendMenuW, CreatePopupMenu, CreateWindowExW, DestroyMenu, DestroyWindow,
            GetSystemMetrics, SetForegroundWindow, TrackPopupMenuEx,
            MF_POPUP, MF_SEPARATOR, MF_STRING, SM_CYSCREEN,
            TPM_LEFTALIGN, TPM_RETURNCMD, TPM_TOPALIGN, WINDOW_EX_STYLE, WS_POPUP,
        };

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

        // 6. Invisible Anchor Window (required by Win32 TrackPopupMenuEx)
        let hwnd = match CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("STATIC"),
            w!("bit_sr_menu_anchor"),
            WS_POPUP,
            x,
            0,
            0,
            0,
            None,
            None,
            None,
            None,
        ) {
            Ok(h) => h,
            Err(_) => {
                let _ = DestroyMenu(hmenu_root);
                return None;
            }
        };

        let _ = SetForegroundWindow(hwnd);

        // Position on the left side of the screen
        let chosen_y = if y < 0 {
            let screen_h = GetSystemMetrics(SM_CYSCREEN);
            (screen_h - 220) / 2
        } else {
            y
        };

        // 7. Track popup menu with native OS modal menu loop
        let selected_cmd = TrackPopupMenuEx(
            hmenu_root,
            (TPM_LEFTALIGN | TPM_TOPALIGN | TPM_RETURNCMD).0,
            x,
            chosen_y,
            hwnd,
            None,
        );

        let _ = DestroyMenu(hmenu_root);
        let _ = DestroyWindow(hwnd);

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
