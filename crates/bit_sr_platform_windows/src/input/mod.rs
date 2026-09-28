//! Input hooks subsystem module.

pub mod keyboard;
pub mod mouse;

pub use keyboard::{get_current_modifiers, KeyboardHookHandle};
pub use mouse::parse_mouse_event;
