//! Mouse input event structures.

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
