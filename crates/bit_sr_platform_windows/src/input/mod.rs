//! Input hooks subsystem module.

pub mod keyboard;
pub mod mouse;

pub use keyboard::{
    get_current_modifiers, is_input_help_active, set_input_help_active, KeyboardHookHandle,
};
pub use mouse::parse_mouse_event;
