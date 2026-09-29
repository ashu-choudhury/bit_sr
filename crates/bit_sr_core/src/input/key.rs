//! Universal Keyboard Key Definitions.
//! Maps physical and virtual keys to platform-agnostic representations.

use std::fmt;

/// Platform-agnostic keyboard key representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum Key {
    #[default]
    Unknown,

    // Letters A - Z
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,

    // Top-row numbers 0 - 9
    Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9,

    // Function keys F1 - F24
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    F13, F14, F15, F16, F17, F18, F19, F20, F21, F22, F23, F24,

    // Navigation and Cursor Control
    Escape,
    Enter,
    Tab,
    Space,
    Backspace,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    LeftArrow,
    RightArrow,
    UpArrow,
    DownArrow,

    // Locks and System Controls
    CapsLock,
    ScrollLock,
    NumLock,
    PrintScreen,
    Pause,

    // Standard Modifiers
    LeftShift,
    RightShift,
    LeftControl,
    RightControl,
    LeftAlt,
    RightAlt,
    LeftSuper,  // Windows / Command / Meta
    RightSuper,
    Menu,        // Context Menu key / Applications key

    // Numpad Keys
    Numpad0, Numpad1, Numpad2, Numpad3, Numpad4,
    Numpad5, Numpad6, Numpad7, Numpad8, Numpad9,
    NumpadAdd,
    NumpadSubtract,
    NumpadMultiply,
    NumpadDivide,
    NumpadDecimal,
    NumpadEnter,

    // Punctuation and Symbols
    Grave,        // ` ~
    Minus,        // - _
    Equals,       // = +
    LeftBracket,  // [ {
    RightBracket, // ] }
    Backslash,    // \ |
    Semicolon,    // ; :
    Apostrophe,   // ' "
    Comma,        // , <
    Period,       // . >
    Slash,        // / ?

    // Media and Hardware Keys
    VolumeMute,
    VolumeDown,
    VolumeUp,
    MediaNextTrack,
    MediaPrevTrack,
    MediaStop,
    MediaPlayPause,

    // Unmapped / Custom Virtual Key code
    Other(u32),
}

impl Key {
    /// Canonical lowercase identifier used in gesture strings and config files (e.g. "t", "f1", "tab", "left").
    pub fn canonical_name(&self) -> &'static str {
        match self {
            Self::Unknown => "unknown",

            Self::A => "a", Self::B => "b", Self::C => "c", Self::D => "d", Self::E => "e",
            Self::F => "f", Self::G => "g", Self::H => "h", Self::I => "i", Self::J => "j",
            Self::K => "k", Self::L => "l", Self::M => "m", Self::N => "n", Self::O => "o",
            Self::P => "p", Self::Q => "q", Self::R => "r", Self::S => "s", Self::T => "t",
            Self::U => "u", Self::V => "v", Self::W => "w", Self::X => "x", Self::Y => "y",
            Self::Z => "z",

            Self::Num0 => "0", Self::Num1 => "1", Self::Num2 => "2", Self::Num3 => "3",
            Self::Num4 => "4", Self::Num5 => "5", Self::Num6 => "6", Self::Num7 => "7",
            Self::Num8 => "8", Self::Num9 => "9",

            Self::F1 => "f1", Self::F2 => "f2", Self::F3 => "f3", Self::F4 => "f4",
            Self::F5 => "f5", Self::F6 => "f6", Self::F7 => "f7", Self::F8 => "f8",
            Self::F9 => "f9", Self::F10 => "f10", Self::F11 => "f11", Self::F12 => "f12",
            Self::F13 => "f13", Self::F14 => "f14", Self::F15 => "f15", Self::F16 => "f16",
            Self::F17 => "f17", Self::F18 => "f18", Self::F19 => "f19", Self::F20 => "f20",
            Self::F21 => "f21", Self::F22 => "f22", Self::F23 => "f23", Self::F24 => "f24",

            Self::Escape => "escape",
            Self::Enter => "enter",
            Self::Tab => "tab",
            Self::Space => "space",
            Self::Backspace => "backspace",
            Self::Delete => "delete",
            Self::Insert => "insert",
            Self::Home => "home",
            Self::End => "end",
            Self::PageUp => "pageup",
            Self::PageDown => "pagedown",
            Self::LeftArrow => "left",
            Self::RightArrow => "right",
            Self::UpArrow => "up",
            Self::DownArrow => "down",

            Self::CapsLock => "capslock",
            Self::ScrollLock => "scrolllock",
            Self::NumLock => "numlock",
            Self::PrintScreen => "printscreen",
            Self::Pause => "pause",

            Self::LeftShift | Self::RightShift => "shift",
            Self::LeftControl | Self::RightControl => "control",
            Self::LeftAlt | Self::RightAlt => "alt",
            Self::LeftSuper | Self::RightSuper => "super",
            Self::Menu => "applications",

            Self::Numpad0 => "numpad0", Self::Numpad1 => "numpad1", Self::Numpad2 => "numpad2",
            Self::Numpad3 => "numpad3", Self::Numpad4 => "numpad4", Self::Numpad5 => "numpad5",
            Self::Numpad6 => "numpad6", Self::Numpad7 => "numpad7", Self::Numpad8 => "numpad8",
            Self::Numpad9 => "numpad9",
            Self::NumpadAdd => "numpad_plus",
            Self::NumpadSubtract => "numpad_minus",
            Self::NumpadMultiply => "numpad_multiply",
            Self::NumpadDivide => "numpad_divide",
            Self::NumpadDecimal => "numpad_decimal",
            Self::NumpadEnter => "numpad_enter",

            Self::Grave => "grave",
            Self::Minus => "minus",
            Self::Equals => "equals",
            Self::LeftBracket => "leftbracket",
            Self::RightBracket => "rightbracket",
            Self::Backslash => "backslash",
            Self::Semicolon => "semicolon",
            Self::Apostrophe => "apostrophe",
            Self::Comma => "comma",
            Self::Period => "period",
            Self::Slash => "slash",

            Self::VolumeMute => "mute",
            Self::VolumeDown => "volumedown",
            Self::VolumeUp => "volumeup",
            Self::MediaNextTrack => "nexttrack",
            Self::MediaPrevTrack => "prevtrack",
            Self::MediaStop => "stop",
            Self::MediaPlayPause => "playpause",

            Self::Other(_) => "other",
        }
    }

    /// Spoken human-friendly display name for screen reader output (e.g. "T", "Left Arrow", "Page Up", "Escape").
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",

            Self::A => "A", Self::B => "B", Self::C => "C", Self::D => "D", Self::E => "E",
            Self::F => "F", Self::G => "G", Self::H => "H", Self::I => "I", Self::J => "J",
            Self::K => "K", Self::L => "L", Self::M => "M", Self::N => "N", Self::O => "O",
            Self::P => "P", Self::Q => "Q", Self::R => "R", Self::S => "S", Self::T => "T",
            Self::U => "U", Self::V => "V", Self::W => "W", Self::X => "X", Self::Y => "Y",
            Self::Z => "Z",

            Self::Num0 => "0", Self::Num1 => "1", Self::Num2 => "2", Self::Num3 => "3",
            Self::Num4 => "4", Self::Num5 => "5", Self::Num6 => "6", Self::Num7 => "7",
            Self::Num8 => "8", Self::Num9 => "9",

            Self::F1 => "F1", Self::F2 => "F2", Self::F3 => "F3", Self::F4 => "F4",
            Self::F5 => "F5", Self::F6 => "F6", Self::F7 => "F7", Self::F8 => "F8",
            Self::F9 => "F9", Self::F10 => "F10", Self::F11 => "F11", Self::F12 => "F12",
            Self::F13 => "F13", Self::F14 => "F14", Self::F15 => "F15", Self::F16 => "F16",
            Self::F17 => "F17", Self::F18 => "F18", Self::F19 => "F19", Self::F20 => "F20",
            Self::F21 => "F21", Self::F22 => "F22", Self::F23 => "F24", Self::F24 => "F24",

            Self::Escape => "Escape",
            Self::Enter => "Enter",
            Self::Tab => "Tab",
            Self::Space => "Space",
            Self::Backspace => "Backspace",
            Self::Delete => "Delete",
            Self::Insert => "Insert",
            Self::Home => "Home",
            Self::End => "End",
            Self::PageUp => "Page Up",
            Self::PageDown => "Page Down",
            Self::LeftArrow => "Left Arrow",
            Self::RightArrow => "Right Arrow",
            Self::UpArrow => "Up Arrow",
            Self::DownArrow => "Down Arrow",

            Self::CapsLock => "Caps Lock",
            Self::ScrollLock => "Scroll Lock",
            Self::NumLock => "Num Lock",
            Self::PrintScreen => "Print Screen",
            Self::Pause => "Pause",

            Self::LeftShift | Self::RightShift => "Shift",
            Self::LeftControl | Self::RightControl => "Control",
            Self::LeftAlt | Self::RightAlt => "Alt",
            Self::LeftSuper | Self::RightSuper => "Windows",
            Self::Menu => "Applications",

            Self::Numpad0 => "Numpad 0", Self::Numpad1 => "Numpad 1", Self::Numpad2 => "Numpad 2",
            Self::Numpad3 => "Numpad 3", Self::Numpad4 => "Numpad 4", Self::Numpad5 => "Numpad 5",
            Self::Numpad6 => "Numpad 6", Self::Numpad7 => "Numpad 7", Self::Numpad8 => "Numpad 8",
            Self::Numpad9 => "Numpad 9",
            Self::NumpadAdd => "Numpad Plus",
            Self::NumpadSubtract => "Numpad Minus",
            Self::NumpadMultiply => "Numpad Multiply",
            Self::NumpadDivide => "Numpad Divide",
            Self::NumpadDecimal => "Numpad Decimal",
            Self::NumpadEnter => "Numpad Enter",

            Self::Grave => "Grave",
            Self::Minus => "Minus",
            Self::Equals => "Equals",
            Self::LeftBracket => "Left Bracket",
            Self::RightBracket => "Right Bracket",
            Self::Backslash => "Backslash",
            Self::Semicolon => "Semicolon",
            Self::Apostrophe => "Apostrophe",
            Self::Comma => "Comma",
            Self::Period => "Period",
            Self::Slash => "Slash",

            Self::VolumeMute => "Mute",
            Self::VolumeDown => "Volume Down",
            Self::VolumeUp => "Volume Up",
            Self::MediaNextTrack => "Next Track",
            Self::MediaPrevTrack => "Previous Track",
            Self::MediaStop => "Stop",
            Self::MediaPlayPause => "Play Pause",

            Self::Other(_) => "Unknown Key",
        }
    }

    /// Checks if this key is a standard keyboard modifier or lock key.
    pub fn is_modifier(&self) -> bool {
        matches!(
            self,
            Self::LeftShift
                | Self::RightShift
                | Self::LeftControl
                | Self::RightControl
                | Self::LeftAlt
                | Self::RightAlt
                | Self::LeftSuper
                | Self::RightSuper
                | Self::CapsLock
                | Self::Insert
        )
    }

    /// Checks if this key represents an alphanumeric character (A-Z or 0-9).
    pub fn is_alphanumeric(&self) -> bool {
        matches!(
            self,
            Self::A
                | Self::B
                | Self::C
                | Self::D
                | Self::E
                | Self::F
                | Self::G
                | Self::H
                | Self::I
                | Self::J
                | Self::K
                | Self::L
                | Self::M
                | Self::N
                | Self::O
                | Self::P
                | Self::Q
                | Self::R
                | Self::S
                | Self::T
                | Self::U
                | Self::V
                | Self::W
                | Self::X
                | Self::Y
                | Self::Z
                | Self::Num0
                | Self::Num1
                | Self::Num2
                | Self::Num3
                | Self::Num4
                | Self::Num5
                | Self::Num6
                | Self::Num7
                | Self::Num8
                | Self::Num9
        )
    }

    /// Parses a case-insensitive key name with common aliases into a Key enum.
    pub fn from_name(name: &str) -> Option<Self> {
        let clean = name.trim().to_lowercase();
        match clean.as_str() {
            "a" => Some(Self::A), "b" => Some(Self::B), "c" => Some(Self::C), "d" => Some(Self::D),
            "e" => Some(Self::E), "f" => Some(Self::F), "g" => Some(Self::G), "h" => Some(Self::H),
            "i" => Some(Self::I), "j" => Some(Self::J), "k" => Some(Self::K), "l" => Some(Self::L),
            "m" => Some(Self::M), "n" => Some(Self::N), "o" => Some(Self::O), "p" => Some(Self::P),
            "q" => Some(Self::Q), "r" => Some(Self::R), "s" => Some(Self::S), "t" => Some(Self::T),
            "u" => Some(Self::U), "v" => Some(Self::V), "w" => Some(Self::W), "x" => Some(Self::X),
            "y" => Some(Self::Y), "z" => Some(Self::Z),

            "0" => Some(Self::Num0), "1" => Some(Self::Num1), "2" => Some(Self::Num2),
            "3" => Some(Self::Num3), "4" => Some(Self::Num4), "5" => Some(Self::Num5),
            "6" => Some(Self::Num6), "7" => Some(Self::Num7), "8" => Some(Self::Num8),
            "9" => Some(Self::Num9),

            "f1" => Some(Self::F1), "f2" => Some(Self::F2), "f3" => Some(Self::F3),
            "f4" => Some(Self::F4), "f5" => Some(Self::F5), "f6" => Some(Self::F6),
            "f7" => Some(Self::F7), "f8" => Some(Self::F8), "f9" => Some(Self::F9),
            "f10" => Some(Self::F10), "f11" => Some(Self::F11), "f12" => Some(Self::F12),
            "f13" => Some(Self::F13), "f14" => Some(Self::F14), "f15" => Some(Self::F15),
            "f16" => Some(Self::F16), "f17" => Some(Self::F17), "f18" => Some(Self::F18),
            "f19" => Some(Self::F19), "f20" => Some(Self::F20), "f21" => Some(Self::F21),
            "f22" => Some(Self::F22), "f23" => Some(Self::F23), "f24" => Some(Self::F24),

            "esc" | "escape" => Some(Self::Escape),
            "return" | "enter" => Some(Self::Enter),
            "tab" => Some(Self::Tab),
            "space" | " " => Some(Self::Space),
            "backspace" | "back" => Some(Self::Backspace),
            "del" | "delete" => Some(Self::Delete),
            "ins" | "insert" => Some(Self::Insert),
            "home" => Some(Self::Home),
            "end" => Some(Self::End),
            "pageup" | "pgup" => Some(Self::PageUp),
            "pagedown" | "pgdn" => Some(Self::PageDown),
            "left" | "leftarrow" | "arrowleft" => Some(Self::LeftArrow),
            "right" | "rightarrow" | "arrowright" => Some(Self::RightArrow),
            "up" | "uparrow" | "arrowup" => Some(Self::UpArrow),
            "down" | "downarrow" | "arrowdown" => Some(Self::DownArrow),

            "caps" | "capslock" | "capital" => Some(Self::CapsLock),
            "scroll" | "scrolllock" => Some(Self::ScrollLock),
            "numlock" => Some(Self::NumLock),
            "printscreen" | "prtscn" => Some(Self::PrintScreen),
            "pause" => Some(Self::Pause),

            "shift" => Some(Self::LeftShift),
            "control" | "ctrl" => Some(Self::LeftControl),
            "alt" => Some(Self::LeftAlt),
            "win" | "windows" | "super" | "meta" => Some(Self::LeftSuper),
            "apps" | "applications" | "menu" => Some(Self::Menu),

            "numpad0" => Some(Self::Numpad0), "numpad1" => Some(Self::Numpad1),
            "numpad2" => Some(Self::Numpad2), "numpad3" => Some(Self::Numpad3),
            "numpad4" => Some(Self::Numpad4), "numpad5" => Some(Self::Numpad5),
            "numpad6" => Some(Self::Numpad6), "numpad7" => Some(Self::Numpad7),
            "numpad8" => Some(Self::Numpad8), "numpad9" => Some(Self::Numpad9),
            "numpad_plus" | "numpad_add" | "numplus" => Some(Self::NumpadAdd),
            "numpad_minus" | "numpad_subtract" | "numminus" => Some(Self::NumpadSubtract),
            "numpad_multiply" | "nummultiply" => Some(Self::NumpadMultiply),
            "numpad_divide" | "numdivide" => Some(Self::NumpadDivide),
            "numpad_decimal" | "numdecimal" | "numpad_period" => Some(Self::NumpadDecimal),
            "numpad_enter" | "numenter" => Some(Self::NumpadEnter),

            "`" | "~" | "grave" => Some(Self::Grave),
            "-" | "_" | "minus" => Some(Self::Minus),
            "=" | "+" | "equals" => Some(Self::Equals),
            "[" | "{" | "leftbracket" => Some(Self::LeftBracket),
            "]" | "}" | "rightbracket" => Some(Self::RightBracket),
            "\\" | "|" | "backslash" => Some(Self::Backslash),
            ";" | ":" | "semicolon" => Some(Self::Semicolon),
            "'" | "\"" | "apostrophe" | "quote" => Some(Self::Apostrophe),
            "," | "<" | "comma" => Some(Self::Comma),
            "." | ">" | "period" | "dot" => Some(Self::Period),
            "/" | "?" | "slash" => Some(Self::Slash),

            "mute" | "volumemute" => Some(Self::VolumeMute),
            "volumedown" => Some(Self::VolumeDown),
            "volumeup" => Some(Self::VolumeUp),
            "nexttrack" => Some(Self::MediaNextTrack),
            "prevtrack" => Some(Self::MediaPrevTrack),
            "stop" => Some(Self::MediaStop),
            "playpause" => Some(Self::MediaPlayPause),

            _ => None,
        }
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_canonical_and_display_names() {
        assert_eq!(Key::T.canonical_name(), "t");
        assert_eq!(Key::T.display_name(), "T");
        assert_eq!(Key::LeftArrow.canonical_name(), "left");
        assert_eq!(Key::LeftArrow.display_name(), "Left Arrow");
        assert_eq!(Key::CapsLock.canonical_name(), "capslock");
        assert_eq!(Key::CapsLock.display_name(), "Caps Lock");
    }

    #[test]
    fn test_key_from_name_parsing() {
        assert_eq!(Key::from_name("t"), Some(Key::T));
        assert_eq!(Key::from_name("T"), Some(Key::T));
        assert_eq!(Key::from_name("esc"), Some(Key::Escape));
        assert_eq!(Key::from_name("escape"), Some(Key::Escape));
        assert_eq!(Key::from_name("leftarrow"), Some(Key::LeftArrow));
        assert_eq!(Key::from_name("left"), Some(Key::LeftArrow));
        assert_eq!(Key::from_name("capslock"), Some(Key::CapsLock));
        assert_eq!(Key::from_name("caps"), Some(Key::CapsLock));
        assert_eq!(Key::from_name("f12"), Some(Key::F12));
        assert_eq!(Key::from_name("invalid_key_xyz"), None);
    }

    #[test]
    fn test_is_modifier() {
        assert!(Key::LeftControl.is_modifier());
        assert!(Key::CapsLock.is_modifier());
        assert!(Key::Insert.is_modifier());
        assert!(!Key::T.is_modifier());
        assert!(!Key::Escape.is_modifier());
    }
}
