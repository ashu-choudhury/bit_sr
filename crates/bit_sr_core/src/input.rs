//! Low-Level Keyboard and Mouse Input Event Models.

use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct KeyModifiers: u32 {
        const SHIFT    = 1 << 0;
        const CONTROL  = 1 << 1;
        const ALT      = 1 << 2;
        const SUPER    = 1 << 3; // Windows Key / Command / Meta
        const CAPSLOCK = 1 << 4;
        const INSERT   = 1 << 5;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyAction {
    Down,
    Up,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEvent {
    pub vk_code: u32,
    pub scan_code: u32,
    pub is_extended: bool,
    pub is_injected: bool,
    pub action: KeyAction,
    pub modifiers: KeyModifiers,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseAction {
    Move,
    LeftDown,
    LeftUp,
    RightDown,
    RightUp,
    MiddleDown,
    MiddleUp,
    Wheel,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MouseEvent {
    pub x: i32,
    pub y: i32,
    pub action: MouseAction,
    pub is_injected: bool,
}
