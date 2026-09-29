//! Left-Side Screen Reader Menu (`SR + M`).
//!
//! Provides a modern, accessible menu docked to the left side of the screen.
//! Focus lands naturally on the menu items with zero artificial speech hacks,
//! allowing Windows UI Automation / AccessKit to announce focus changes natively.

use crate::MenuItemData;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuAction {
    OpenSettings,
    OpenSpeechSettings,
    OpenKeyboardSettings,
    OpenPluginManager,
    ViewLog,
    ReloadPlugins,
    SetSpeechModeTalk,
    SetSpeechModeMute,
    ToggleInputHelp,
    About,
    Restart,
    Quit,
}

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

/// Displays an authentic native Windows popup menu (identical to NVDA's wx.Menu.PopupMenu)
/// docked to the left side of the screen.
///
/// Because this is an authentic Win32 `#32768` popup menu:
/// - It does NOT open a separate application window.
/// - Every screen reader (NVDA, Narrator, JAWS, bit_sr) natively and flawlessly
///   announces items, submenus, and keyboard accelerators out of the box.
#[cfg(windows)]
pub fn show_native_popup_menu(x: i32, y: i32) -> Option<MenuAction> {
    unsafe {
        use windows::core::w;
        use windows::Win32::UI::WindowsAndMessaging::{
            AppendMenuW, CreatePopupMenu, CreateWindowExW, DestroyMenu, DestroyWindow,
            GetSystemMetrics, SetForegroundWindow, TrackPopupMenuEx,
            MF_POPUP, MF_SEPARATOR, MF_STRING, SM_CYSCREEN,
            TPM_LEFTALIGN, TPM_RETURNCMD, TPM_TOPALIGN, WINDOW_EX_STYLE, WS_POPUP,
        };

        // 1. Create Preferences Submenu
        let hmenu_pref = CreatePopupMenu().ok()?;
        let _ = AppendMenuW(hmenu_pref, MF_STRING, ID_PREF_SETTINGS as usize, w!("&Settings...\tSR+Ctrl+S"));
        let _ = AppendMenuW(hmenu_pref, MF_STRING, ID_PREF_SPEECH as usize, w!("S&peech & Voices..."));
        let _ = AppendMenuW(hmenu_pref, MF_STRING, ID_PREF_KEYBOARD as usize, w!("&Keyboard Shortcuts..."));

        // 2. Create Tools Submenu
        let hmenu_tools = CreatePopupMenu().ok()?;
        let _ = AppendMenuW(hmenu_tools, MF_STRING, ID_TOOLS_PLUGINS as usize, w!("&Extension & Plugin Manager..."));
        let _ = AppendMenuW(hmenu_tools, MF_STRING, ID_TOOLS_LOG as usize, w!("View &Diagnostic Log"));
        let _ = AppendMenuW(hmenu_tools, MF_STRING, ID_TOOLS_RELOAD as usize, w!("&Reload Extensions"));

        // 3. Create Speech Mode Submenu
        let hmenu_speech = CreatePopupMenu().ok()?;
        let _ = AppendMenuW(hmenu_speech, MF_STRING, ID_SPEECH_TALK as usize, w!("&Talk (Full Speech)"));
        let _ = AppendMenuW(hmenu_speech, MF_STRING, ID_SPEECH_MUTE as usize, w!("&Mute (Silent)"));

        // 4. Create Help Submenu
        let hmenu_help = CreatePopupMenu().ok()?;
        let _ = AppendMenuW(hmenu_help, MF_STRING, ID_HELP_INPUT_HELP as usize, w!("&Input Help Mode\tSR+1"));
        let _ = AppendMenuW(hmenu_help, MF_STRING, ID_HELP_ABOUT as usize, w!("&About bit_sr..."));

        // 5. Create Root Menu
        let hmenu_root = CreatePopupMenu().ok()?;
        let _ = AppendMenuW(hmenu_root, MF_POPUP, hmenu_pref.0 as usize, w!("&Preferences"));
        let _ = AppendMenuW(hmenu_root, MF_POPUP, hmenu_tools.0 as usize, w!("&Tools"));
        let _ = AppendMenuW(hmenu_root, MF_POPUP, hmenu_speech.0 as usize, w!("Speech &Mode"));
        let _ = AppendMenuW(hmenu_root, MF_POPUP, hmenu_help.0 as usize, w!("&Help"));
        let _ = AppendMenuW(hmenu_root, MF_SEPARATOR, 0, None);
        let _ = AppendMenuW(hmenu_root, MF_STRING, ID_RESTART as usize, w!("&Restart bit_sr"));
        let _ = AppendMenuW(hmenu_root, MF_STRING, ID_EXIT as usize, w!("E&xit bit_sr\tSR+Q"));

        // 6. Create lightweight hidden anchor window to host the menu
        let hwnd = match CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("bit_sr_menu_anchor"),
            WS_POPUP,
            x,
            y,
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

        // Cleanup menu and anchor window
        let _ = DestroyMenu(hmenu_root);
        let _ = DestroyWindow(hwnd);

        // 8. Map selected ID to MenuAction
        map_menu_cmd_to_action(selected_cmd.0 as u32)
    }
}

/// Maps native Win32 popup menu command IDs to `MenuAction`.
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

pub fn get_main_menu_items() -> Vec<MenuItemData> {
    vec![
        MenuItemData {
            id: "pref".into(),
            label: "Preferences".into(),
            shortcut: "".into(),
            has_submenu: true,
            icon: "⚙️".into(),
        },
        MenuItemData {
            id: "tools".into(),
            label: "Tools".into(),
            shortcut: "".into(),
            has_submenu: true,
            icon: "🛠️".into(),
        },
        MenuItemData {
            id: "speech_mode".into(),
            label: "Speech Mode".into(),
            shortcut: "".into(),
            has_submenu: true,
            icon: "🔊".into(),
        },
        MenuItemData {
            id: "help".into(),
            label: "Help".into(),
            shortcut: "".into(),
            has_submenu: true,
            icon: "❓".into(),
        },
        MenuItemData {
            id: "restart".into(),
            label: "Restart bit_sr".into(),
            shortcut: "".into(),
            has_submenu: false,
            icon: "🔄".into(),
        },
        MenuItemData {
            id: "exit".into(),
            label: "Exit bit_sr".into(),
            shortcut: "SR + Q".into(),
            has_submenu: false,
            icon: "🚪".into(),
        },
    ]
}

pub fn get_submenu_items(submenu_id: &str) -> (String, Vec<MenuItemData>) {
    match submenu_id {
        "pref" => (
            "Preferences".into(),
            vec![
                MenuItemData {
                    id: "pref.settings".into(),
                    label: "Settings...".into(),
                    shortcut: "SR + Ctrl + S".into(),
                    has_submenu: false,
                    icon: "⚙️".into(),
                },
                MenuItemData {
                    id: "pref.speech".into(),
                    label: "Speech & Voices...".into(),
                    shortcut: "".into(),
                    has_submenu: false,
                    icon: "🔊".into(),
                },
                MenuItemData {
                    id: "pref.keyboard".into(),
                    label: "Keyboard Shortcuts...".into(),
                    shortcut: "".into(),
                    has_submenu: false,
                    icon: "⌨️".into(),
                },
            ],
        ),
        "tools" => (
            "Tools".into(),
            vec![
                MenuItemData {
                    id: "tools.plugins".into(),
                    label: "Extension & Plugin Manager...".into(),
                    shortcut: "".into(),
                    has_submenu: false,
                    icon: "🧩".into(),
                },
                MenuItemData {
                    id: "tools.log".into(),
                    label: "View Diagnostic Log".into(),
                    shortcut: "".into(),
                    has_submenu: false,
                    icon: "📋".into(),
                },
                MenuItemData {
                    id: "tools.reload".into(),
                    label: "Reload Extensions".into(),
                    shortcut: "".into(),
                    has_submenu: false,
                    icon: "🔄".into(),
                },
            ],
        ),
        "speech_mode" => (
            "Speech Mode".into(),
            vec![
                MenuItemData {
                    id: "speech.talk".into(),
                    label: "Talk (Full Speech)".into(),
                    shortcut: "".into(),
                    has_submenu: false,
                    icon: "🗣️".into(),
                },
                MenuItemData {
                    id: "speech.mute".into(),
                    label: "Mute (Silent)".into(),
                    shortcut: "".into(),
                    has_submenu: false,
                    icon: "🔇".into(),
                },
            ],
        ),
        "help" => (
            "Help".into(),
            vec![
                MenuItemData {
                    id: "help.input_help".into(),
                    label: "Input Help Mode".into(),
                    shortcut: "SR + 1".into(),
                    has_submenu: false,
                    icon: "ℹ️".into(),
                },
                MenuItemData {
                    id: "help.about".into(),
                    label: "About bit_sr...".into(),
                    shortcut: "".into(),
                    has_submenu: false,
                    icon: "⭐".into(),
                },
            ],
        ),
        _ => ("bit_sr".into(), get_main_menu_items()),
    }
}

pub fn parse_action(id: &str) -> Option<MenuAction> {
    match id {
        "pref.settings" => Some(MenuAction::OpenSettings),
        "pref.speech" => Some(MenuAction::OpenSpeechSettings),
        "pref.keyboard" => Some(MenuAction::OpenKeyboardSettings),
        "tools.plugins" => Some(MenuAction::OpenPluginManager),
        "tools.log" => Some(MenuAction::ViewLog),
        "tools.reload" => Some(MenuAction::ReloadPlugins),
        "speech.talk" => Some(MenuAction::SetSpeechModeTalk),
        "speech.mute" => Some(MenuAction::SetSpeechModeMute),
        "help.input_help" => Some(MenuAction::ToggleInputHelp),
        "help.about" => Some(MenuAction::About),
        "restart" => Some(MenuAction::Restart),
        "exit" => Some(MenuAction::Quit),
        _ => None,
    }
}
