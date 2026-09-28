//! Low-Level Keyboard and Mouse Input Event Models.

pub mod gesture;
pub mod key;
pub mod mouse;

pub use gesture::{
    GestureParseError, InputGesture, KeyModifiers, SRKeyAction, SRKeyConfig, SRModifierTracker,
};
pub use key::Key;
pub use mouse::{MouseAction, MouseEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyAction {
    Down,
    Up,
}

/// A physical or synthesized keyboard event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEvent {
    pub key: Key,
    pub vk_code: u32,
    pub scan_code: u32,
    pub is_extended: bool,
    pub is_injected: bool,
    pub action: KeyAction,
    pub modifiers: KeyModifiers,
    pub text: Option<String>,
}

impl KeyEvent {
    /// Helper to construct a clean KeyEvent with Key, Action, and Modifiers.
    pub fn new(key: Key, action: KeyAction, modifiers: KeyModifiers) -> Self {
        Self {
            key,
            vk_code: 0,
            scan_code: 0,
            is_extended: false,
            is_injected: false,
            action,
            modifiers,
            text: None,
        }
    }

    /// Converts this key event into a normalized InputGesture if it is a KeyDown action.
    pub fn to_gesture(&self) -> Option<InputGesture> {
        if self.action == KeyAction::Down {
            Some(InputGesture::new(self.modifiers, self.key))
        } else {
            None
        }
    }
}
