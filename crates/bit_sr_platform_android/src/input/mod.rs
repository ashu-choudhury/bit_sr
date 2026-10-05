//! Android Input Subsystem: Raw Touch Processing and Hardware Keys.

pub mod keyboard;
pub mod touch;

pub use keyboard::{HardwareKeyTracker, KeyAction};
pub use touch::{
    SwipeDirection, TouchResult, TouchStateMachine, ACTION_CANCEL, ACTION_DOWN, ACTION_MOVE,
    ACTION_UP,
};
