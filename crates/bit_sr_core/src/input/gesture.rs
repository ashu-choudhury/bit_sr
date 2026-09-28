//! Input Gesture Normalization, SR Modifier Abstraction, and Shortcut Parsing.
//! Inspired by NVDA's input gesture architecture with sub-millisecond, zero-allocation Rust ergonomics.

use super::key::Key;
use bitflags::bitflags;
use std::fmt;
use std::time::Instant;

bitflags! {
    /// Keyboard modifier flags including the abstract Screen Reader (SR) modifier.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
    pub struct KeyModifiers: u32 {
        const SHIFT    = 1 << 0;
        const CONTROL  = 1 << 1;
        const ALT      = 1 << 2;
        const SUPER    = 1 << 3; // Windows Key / Super / Meta
        const SR       = 1 << 4; // Screen Reader Modifier (CapsLock or Insert)
        const CAPSLOCK = 1 << 5; // Physical CapsLock active
        const INSERT   = 1 << 6; // Physical Insert active
    }
}

impl KeyModifiers {
    /// Normalizes physical modifiers into abstract general modifiers.
    pub fn generalize(&self) -> Self {
        let mut generalized = Self::empty();
        if self.contains(Self::CONTROL) {
            generalized |= Self::CONTROL;
        }
        if self.contains(Self::ALT) {
            generalized |= Self::ALT;
        }
        if self.contains(Self::SHIFT) {
            generalized |= Self::SHIFT;
        }
        if self.contains(Self::SUPER) {
            generalized |= Self::SUPER;
        }
        // SR and Insert modifiers generalize to SR
        if self.contains(Self::SR) || self.contains(Self::INSERT) {
            generalized |= Self::SR;
        }
        generalized
    }
}

/// Normalized representation of a physical or abstracted user input gesture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct InputGesture {
    pub modifiers: KeyModifiers,
    pub key: Key,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GestureParseError {
    EmptyString,
    MissingKey,
    UnknownModifier(String),
    UnknownKey(String),
}

impl fmt::Display for GestureParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyString => write!(f, "gesture string cannot be empty"),
            Self::MissingKey => write!(f, "gesture string is missing a primary key"),
            Self::UnknownModifier(m) => write!(f, "unknown modifier: '{m}'"),
            Self::UnknownKey(k) => write!(f, "unknown key: '{k}'"),
        }
    }
}

impl std::error::Error for GestureParseError {}

impl InputGesture {
    /// Creates a new InputGesture with automatically generalized modifiers.
    pub fn new(modifiers: KeyModifiers, key: Key) -> Self {
        let mut generalized = modifiers.generalize();
        // Discard modifier from modifiers set if the primary key IS that modifier
        // to avoid "Control + Control", "Shift + Shift", "Alt + Alt", etc.
        match key {
            Key::LeftControl | Key::RightControl => generalized.remove(KeyModifiers::CONTROL),
            Key::LeftAlt | Key::RightAlt => generalized.remove(KeyModifiers::ALT),
            Key::LeftShift | Key::RightShift => generalized.remove(KeyModifiers::SHIFT),
            Key::LeftSuper | Key::RightSuper => generalized.remove(KeyModifiers::SUPER),
            Key::CapsLock | Key::Insert => generalized.remove(KeyModifiers::SR),
            _ => {}
        }

        Self {
            modifiers: generalized,
            key,
        }
    }

    /// Normalized, deterministic canonical string ID (e.g. "sr+t", "ctrl+alt+q", "f1").
    /// Always follows the canonical order: ctrl -> alt -> shift -> win -> sr -> <key>.
    pub fn canonical_id(&self) -> String {
        let mut parts = Vec::new();
        if self.modifiers.contains(KeyModifiers::CONTROL) {
            parts.push("ctrl");
        }
        if self.modifiers.contains(KeyModifiers::ALT) {
            parts.push("alt");
        }
        if self.modifiers.contains(KeyModifiers::SHIFT) {
            parts.push("shift");
        }
        if self.modifiers.contains(KeyModifiers::SUPER) {
            parts.push("win");
        }
        if self.modifiers.contains(KeyModifiers::SR) {
            parts.push("sr");
        }
        let key_name = match self.key {
            Key::CapsLock => "sr",
            _ => self.key.canonical_name(),
        };
        parts.push(key_name);
        parts.join("+")
    }

    /// Human-friendly spoken display name for speech output and UI (e.g. "SR + T", "Control + Alt + Q").
    pub fn display_name(&self) -> String {
        let mut parts = Vec::new();
        if self.modifiers.contains(KeyModifiers::CONTROL) {
            parts.push("Control");
        }
        if self.modifiers.contains(KeyModifiers::ALT) {
            parts.push("Alt");
        }
        if self.modifiers.contains(KeyModifiers::SHIFT) {
            parts.push("Shift");
        }
        if self.modifiers.contains(KeyModifiers::SUPER) {
            parts.push("Windows");
        }
        if self.modifiers.contains(KeyModifiers::SR) {
            parts.push("SR");
        }

        // When the primary key is CapsLock (the default SR key), speak "SR"
        let key_name = match self.key {
            Key::CapsLock => "SR",
            _ => self.key.display_name(),
        };
        parts.push(key_name);
        parts.join(" + ")
    }

    /// Parses a user-configurable gesture string (e.g. "sr+t", "ctrl+alt+q", "insert+t", "capslock+s").
    pub fn parse(s: &str) -> Result<Self, GestureParseError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(GestureParseError::EmptyString);
        }

        let tokens: Vec<&str> = trimmed.split('+').map(|t| t.trim()).collect();
        if tokens.is_empty() {
            return Err(GestureParseError::EmptyString);
        }

        let mut modifiers = KeyModifiers::empty();
        let key_token = tokens[tokens.len() - 1];

        // Process modifiers (all tokens except the last one)
        for &tok in &tokens[..tokens.len() - 1] {
            let lower = tok.to_lowercase();
            match lower.as_str() {
                "ctrl" | "control" => modifiers |= KeyModifiers::CONTROL,
                "alt" => modifiers |= KeyModifiers::ALT,
                "shift" => modifiers |= KeyModifiers::SHIFT,
                "win" | "windows" | "super" | "meta" => modifiers |= KeyModifiers::SUPER,
                "sr" | "nvda" | "capslock" | "caps" | "insert" | "ins" => {
                    modifiers |= KeyModifiers::SR;
                }
                _ => return Err(GestureParseError::UnknownModifier(tok.to_string())),
            }
        }

        // The final token is the primary key
        let key = Key::from_name(key_token)
            .ok_or_else(|| GestureParseError::UnknownKey(key_token.to_string()))?;

        Ok(Self { modifiers, key })
    }
}

impl fmt::Display for InputGesture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.canonical_id())
    }
}

/// Configuration specifying which physical keys act as the Screen Reader (SR) modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SRKeyConfig {
    /// Whether CapsLock acts as the SR modifier (default: true)
    pub use_caps_lock: bool,
    /// Whether Insert acts as the SR modifier (default: false)
    pub use_insert: bool,
    /// Whether Numpad Insert (0) acts as the SR modifier (default: false)
    pub use_numpad_insert: bool,
    /// Maximum delay in milliseconds between two taps to toggle CapsLock on/off (default: 350ms)
    pub double_tap_timeout_ms: u64,
}

impl Default for SRKeyConfig {
    fn default() -> Self {
        Self {
            use_caps_lock: true,
            use_insert: false,
            use_numpad_insert: false,
            double_tap_timeout_ms: 350,
        }
    }
}

/// Action resulting from an SR modifier key press or release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SRKeyAction {
    /// Normal non-modifier key event, pass to application or shortcut evaluator.
    None,
    /// Key intercepted as SR modifier (must be trapped from the active OS window).
    InterceptModifier,
    /// Double-tap detected on CapsLock! Signal OS to toggle hardware CapsLock on/off.
    ToggleCapsLock,
}

/// State tracker for the SR modifier and CapsLock double-tap timing.
#[derive(Debug, Clone)]
pub struct SRModifierTracker {
    pub config: SRKeyConfig,
    is_sr_held: bool,
    combo_used: bool,
    last_solitary_release: Option<Instant>,
    caps_lock_active: bool,
}

impl SRModifierTracker {
    pub fn new(config: SRKeyConfig) -> Self {
        Self {
            config,
            is_sr_held: false,
            combo_used: false,
            last_solitary_release: None,
            caps_lock_active: false,
        }
    }

    /// Checks if a physical key is currently configured as an SR modifier key.
    pub fn is_sr_key(&self, key: Key) -> bool {
        match key {
            Key::CapsLock => self.config.use_caps_lock,
            Key::Insert => self.config.use_insert,
            Key::Numpad0 => self.config.use_numpad_insert,
            _ => false,
        }
    }

    pub fn is_sr_held(&self) -> bool {
        self.is_sr_held
    }

    pub fn is_caps_lock_active(&self) -> bool {
        self.caps_lock_active
    }

    pub fn set_caps_lock_active(&mut self, active: bool) {
        self.caps_lock_active = active;
    }

    /// Processes a KeyDown event.
    pub fn on_key_down(&mut self, key: Key, now: Instant) -> SRKeyAction {
        if self.is_sr_key(key) {
            self.is_sr_held = true;
            self.combo_used = false;

            // CapsLock double-tap detection
            if key == Key::CapsLock && self.config.use_caps_lock {
                if let Some(release_time) = self.last_solitary_release {
                    let elapsed = now.duration_since(release_time).as_millis() as u64;
                    if elapsed <= self.config.double_tap_timeout_ms {
                        // Double-tap confirmed! Toggle CapsLock state
                        self.last_solitary_release = None;
                        self.caps_lock_active = !self.caps_lock_active;
                        return SRKeyAction::ToggleCapsLock;
                    }
                }
            }

            SRKeyAction::InterceptModifier
        } else {
            if self.is_sr_held {
                // Another key was pressed while SR was held down
                self.combo_used = true;
                self.last_solitary_release = None;
            }
            SRKeyAction::None
        }
    }

    /// Processes a KeyUp event.
    pub fn on_key_up(&mut self, key: Key, now: Instant) -> SRKeyAction {
        if self.is_sr_key(key) {
            self.is_sr_held = false;
            if !self.combo_used && key == Key::CapsLock && self.config.use_caps_lock {
                // Solitary tap & release: start double-tap window
                self.last_solitary_release = Some(now);
            } else {
                self.last_solitary_release = None;
            }
            SRKeyAction::InterceptModifier
        } else {
            SRKeyAction::None
        }
    }

    /// Resets all internal tracking states.
    pub fn reset(&mut self) {
        self.is_sr_held = false;
        self.combo_used = false;
        self.last_solitary_release = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_gesture_canonical_id() {
        let g1 = InputGesture::new(KeyModifiers::SR, Key::T);
        assert_eq!(g1.canonical_id(), "sr+t");

        let g2 = InputGesture::new(KeyModifiers::CONTROL | KeyModifiers::ALT, Key::Q);
        assert_eq!(g2.canonical_id(), "ctrl+alt+q");

        let g3 = InputGesture::new(KeyModifiers::empty(), Key::F1);
        assert_eq!(g3.canonical_id(), "f1");

        let g4 = InputGesture::new(KeyModifiers::SR | KeyModifiers::CONTROL, Key::UpArrow);
        assert_eq!(g4.canonical_id(), "ctrl+sr+up");
    }

    #[test]
    fn test_gesture_display_name() {
        let g1 = InputGesture::new(KeyModifiers::SR, Key::T);
        assert_eq!(g1.display_name(), "SR + T");

        let g2 = InputGesture::new(KeyModifiers::CONTROL | KeyModifiers::ALT, Key::Q);
        assert_eq!(g2.display_name(), "Control + Alt + Q");

        let g3 = InputGesture::new(KeyModifiers::empty(), Key::Escape);
        assert_eq!(g3.display_name(), "Escape");

        // Modifier keys alone do not duplicate their name (e.g. "Control" instead of "Control + Control")
        let g_ctrl = InputGesture::new(KeyModifiers::CONTROL, Key::LeftControl);
        assert_eq!(g_ctrl.display_name(), "Control");

        let g_shift = InputGesture::new(KeyModifiers::SHIFT, Key::RightShift);
        assert_eq!(g_shift.display_name(), "Shift");

        let g_sr = InputGesture::new(KeyModifiers::SR, Key::CapsLock);
        assert_eq!(g_sr.display_name(), "SR");
    }

    #[test]
    fn test_gesture_parse() {
        let parsed = InputGesture::parse("sr+t").unwrap();
        assert_eq!(parsed, InputGesture::new(KeyModifiers::SR, Key::T));

        let parsed_caps = InputGesture::parse("capslock+s").unwrap();
        assert_eq!(parsed_caps, InputGesture::new(KeyModifiers::SR, Key::S));

        let parsed_insert = InputGesture::parse("insert+tab").unwrap();
        assert_eq!(parsed_insert, InputGesture::new(KeyModifiers::SR, Key::Tab));

        let parsed_ctrl_alt = InputGesture::parse("ctrl+alt+q").unwrap();
        assert_eq!(
            parsed_ctrl_alt,
            InputGesture::new(KeyModifiers::CONTROL | KeyModifiers::ALT, Key::Q)
        );

        let parsed_single = InputGesture::parse("f12").unwrap();
        assert_eq!(parsed_single, InputGesture::new(KeyModifiers::empty(), Key::F12));

        assert!(InputGesture::parse("").is_err());
        assert!(InputGesture::parse("invalidmodifier+t").is_err());
        assert!(InputGesture::parse("ctrl+unknownkeyxyz").is_err());
    }

    #[test]
    fn test_sr_tracker_capslock_double_tap() {
        let mut tracker = SRModifierTracker::new(SRKeyConfig::default());
        let t0 = Instant::now();

        // 1. Single tap down & up (solitary tap)
        let act1 = tracker.on_key_down(Key::CapsLock, t0);
        assert_eq!(act1, SRKeyAction::InterceptModifier);
        assert!(tracker.is_sr_held());

        let t1 = t0 + Duration::from_millis(50);
        let act2 = tracker.on_key_up(Key::CapsLock, t1);
        assert_eq!(act2, SRKeyAction::InterceptModifier);
        assert!(!tracker.is_sr_held());
        assert!(!tracker.is_caps_lock_active());

        // 2. Second tap within 200ms -> Double tap detected!
        let t2 = t1 + Duration::from_millis(200);
        let act3 = tracker.on_key_down(Key::CapsLock, t2);
        assert_eq!(act3, SRKeyAction::ToggleCapsLock);
        assert!(tracker.is_caps_lock_active());

        // Up on second tap
        let t3 = t2 + Duration::from_millis(50);
        let act4 = tracker.on_key_up(Key::CapsLock, t3);
        assert_eq!(act4, SRKeyAction::InterceptModifier);

        // 3. Third tap after 500ms should NOT immediately toggle (needs double-tap again)
        let t4 = t3 + Duration::from_millis(500);
        let act5 = tracker.on_key_down(Key::CapsLock, t4);
        assert_eq!(act5, SRKeyAction::InterceptModifier);
    }

    #[test]
    fn test_sr_tracker_combo_does_not_trigger_double_tap() {
        let mut tracker = SRModifierTracker::new(SRKeyConfig::default());
        let t0 = Instant::now();

        // Hold CapsLock
        tracker.on_key_down(Key::CapsLock, t0);

        // Press T while held
        let t1 = t0 + Duration::from_millis(30);
        let act_t = tracker.on_key_down(Key::T, t1);
        assert_eq!(act_t, SRKeyAction::None);

        tracker.on_key_up(Key::T, t1 + Duration::from_millis(40));

        // Release CapsLock
        let t2 = t1 + Duration::from_millis(100);
        tracker.on_key_up(Key::CapsLock, t2);

        // Quick subsequent tap should NOT be a double-tap because CapsLock was used in a combo!
        let t3 = t2 + Duration::from_millis(100);
        let act_subsequent = tracker.on_key_down(Key::CapsLock, t3);
        assert_eq!(act_subsequent, SRKeyAction::InterceptModifier);
    }
}
