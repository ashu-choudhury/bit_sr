//! Hardware Keyboard and Physical Accessibility Shortcut Keys on Android.
//! Handles volume key combinations, DPAD navigation, and hardware keyboard shortcuts.

/// Android keycode constants (from `android.view.KeyEvent`).
pub const KEYCODE_BACK: i32 = 4;
pub const KEYCODE_VOLUME_UP: i32 = 24;
pub const KEYCODE_VOLUME_DOWN: i32 = 25;
pub const KEYCODE_DPAD_UP: i32 = 19;
pub const KEYCODE_DPAD_DOWN: i32 = 20;
pub const KEYCODE_DPAD_LEFT: i32 = 21;
pub const KEYCODE_DPAD_RIGHT: i32 = 22;
pub const KEYCODE_DPAD_CENTER: i32 = 23;
pub const KEYCODE_ENTER: i32 = 66;
pub const KEYCODE_SPACE: i32 = 62;

/// Physical hardware key action (down, up).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    Down,
    Up,
}

/// Tracks physical accessibility shortcut keys (e.g. Volume Up + Volume Down hold).
#[derive(Debug, Default)]
pub struct HardwareKeyTracker {
    vol_up_pressed: bool,
    vol_down_pressed: bool,
    both_down_start: Option<i64>,
}

impl HardwareKeyTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Process a physical key event. Returns `true` if the volume key accessibility shortcut was triggered.
    pub fn process_key(&mut self, keycode: i32, action: KeyAction, time_ms: i64) -> bool {
        match (keycode, action) {
            (KEYCODE_VOLUME_UP, KeyAction::Down) => {
                self.vol_up_pressed = true;
                if self.vol_down_pressed && self.both_down_start.is_none() {
                    self.both_down_start = Some(time_ms);
                }
            }
            (KEYCODE_VOLUME_UP, KeyAction::Up) => {
                self.vol_up_pressed = false;
                self.both_down_start = None;
            }
            (KEYCODE_VOLUME_DOWN, KeyAction::Down) => {
                self.vol_down_pressed = true;
                if self.vol_up_pressed && self.both_down_start.is_none() {
                    self.both_down_start = Some(time_ms);
                }
            }
            (KEYCODE_VOLUME_DOWN, KeyAction::Up) => {
                self.vol_down_pressed = false;
                self.both_down_start = None;
            }
            _ => {}
        }

        // Standard Android accessibility shortcut: both volume keys held for 3000ms
        if let Some(start) = self.both_down_start {
            if time_ms - start >= 3000 {
                self.both_down_start = None;
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_key_shortcut() {
        let mut tracker = HardwareKeyTracker::new();

        tracker.process_key(KEYCODE_VOLUME_UP, KeyAction::Down, 1000);
        assert!(!tracker.process_key(KEYCODE_VOLUME_DOWN, KeyAction::Down, 1050));

        // Held for 2999ms -> not triggered yet
        assert!(!tracker.process_key(KEYCODE_VOLUME_UP, KeyAction::Down, 4000));

        // Held for 3000ms -> triggered!
        assert!(tracker.process_key(KEYCODE_VOLUME_UP, KeyAction::Down, 4050));
    }
}
