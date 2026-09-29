//! Screen Reader Menu Definitions and Actions.
//!
//! Provides platform-agnostic data models for the screen reader popup menu and actions.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// Description of an item in a menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuItemData {
    pub id: String,
    pub label: String,
    pub shortcut: String,
    pub has_submenu: bool,
    pub icon: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_menu_action_variants() {
        let action = MenuAction::OpenSettings;
        assert_eq!(action, MenuAction::OpenSettings);
    }
}
