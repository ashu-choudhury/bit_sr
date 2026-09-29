//! Accessible Native GUI Subsystem for `bit_sr`.
//!
//! Provides the left-side screen reader menu (`SR + M`),
//! the accessible settings dashboard, and AccessKit integration powered by Slint.

slint::include_modules!();

pub mod controller;
pub mod menu;
pub mod settings;

pub use controller::{UiCommand, UiEvent, UiHandle};
pub use menu::MenuAction;
pub use settings::UiSettings;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_menu_items_generation() {
        let items = menu::get_main_menu_items();
        assert!(!items.is_empty());
        assert_eq!(items[0].id, "pref");
        assert!(items[0].has_submenu);

        let (title, sub_items) = menu::get_submenu_items("pref");
        assert_eq!(title, "Preferences");
        assert_eq!(sub_items[0].id, "pref.settings");
    }

    #[test]
    fn test_action_parsing() {
        assert_eq!(menu::parse_action("pref.settings"), Some(MenuAction::OpenSettings));
        assert_eq!(menu::parse_action("exit"), Some(MenuAction::Quit));
        assert_eq!(menu::parse_action("invalid_xyz"), None);
    }

    #[test]
    fn test_native_menu_command_mapping() {
        assert_eq!(menu::map_menu_cmd_to_action(menu::ID_PREF_SETTINGS), Some(MenuAction::OpenSettings));
        assert_eq!(menu::map_menu_cmd_to_action(menu::ID_PREF_SPEECH), Some(MenuAction::OpenSpeechSettings));
        assert_eq!(menu::map_menu_cmd_to_action(menu::ID_PREF_KEYBOARD), Some(MenuAction::OpenKeyboardSettings));
        assert_eq!(menu::map_menu_cmd_to_action(menu::ID_TOOLS_PLUGINS), Some(MenuAction::OpenPluginManager));
        assert_eq!(menu::map_menu_cmd_to_action(menu::ID_SPEECH_TALK), Some(MenuAction::SetSpeechModeTalk));
        assert_eq!(menu::map_menu_cmd_to_action(menu::ID_SPEECH_MUTE), Some(MenuAction::SetSpeechModeMute));
        assert_eq!(menu::map_menu_cmd_to_action(menu::ID_EXIT), Some(MenuAction::Quit));
        assert_eq!(menu::map_menu_cmd_to_action(99999), None);
    }
}
