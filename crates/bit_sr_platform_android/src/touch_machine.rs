//! Native Touch State Machine for Android.
//! Intercepts raw MotionEvents (ACTION_DOWN, ACTION_MOVE, ACTION_UP) directly in Rust,
//! completely bypassing slow AOSP TouchExplorer delay timers.

use bit_sr_engine::commands::ScreenReaderCommand;

/// Android MotionEvent action constants.
pub const ACTION_DOWN: i32 = 0;
pub const ACTION_UP: i32 = 1;
pub const ACTION_MOVE: i32 = 2;
pub const ACTION_CANCEL: i32 = 3;

/// Touch gesture action detected by the native state machine.
#[derive(Debug, Clone, PartialEq)]
pub enum TouchResult {
    /// Finger is moving across the screen (explore by touch).
    Explore { x: f64, y: f64 },
    /// Quick tap completed.
    Tap { x: f64, y: f64 },
    /// Quick double-tap detected (< 250ms interval).
    DoubleTap { x: f64, y: f64 },
    /// Directional swipe / flick detected.
    Flick(ScreenReaderCommand),
    /// Touch event ignored or in progress.
    None,
}

/// Native Touch State Machine.
pub struct TouchStateMachine {
    is_down: bool,
    down_x: f64,
    down_y: f64,
    down_time: i64,
    last_up_time: i64,
    last_up_x: f64,
    last_up_y: f64,
    drag_threshold_sq: f64,
    double_tap_timeout_ms: i64,
    double_tap_distance_sq: f64,
}

impl Default for TouchStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl TouchStateMachine {
    pub fn new() -> Self {
        Self {
            is_down: false,
            down_x: 0.0,
            down_y: 0.0,
            down_time: 0,
            last_up_time: 0,
            last_up_x: 0.0,
            last_up_y: 0.0,
            // 20px threshold for tap vs explore movement
            drag_threshold_sq: 400.0,
            // 250ms double-tap timeout
            double_tap_timeout_ms: 250,
            // 100px allowed slop between consecutive double-tap points
            double_tap_distance_sq: 10000.0,
        }
    }

    /// Processes a raw touch point directly from JNI in microseconds.
    pub fn process_touch(&mut self, action: i32, x: f64, y: f64, time_ms: i64) -> TouchResult {
        match action {
            ACTION_DOWN => {
                self.is_down = true;
                self.down_x = x;
                self.down_y = y;
                self.down_time = time_ms;

                // Check for double-tap
                let time_diff = time_ms - self.last_up_time;
                let dx = x - self.last_up_x;
                let dy = y - self.last_up_y;
                let dist_sq = dx * dx + dy * dy;

                if time_diff <= self.double_tap_timeout_ms && dist_sq <= self.double_tap_distance_sq {
                    // Instant double-tap detected!
                    self.last_up_time = 0;
                    return TouchResult::DoubleTap { x, y };
                }

                TouchResult::None
            }
            ACTION_MOVE => {
                if !self.is_down {
                    return TouchResult::None;
                }

                let dx = x - self.down_x;
                let dy = y - self.down_y;
                let dist_sq = dx * dx + dy * dy;

                // If finger moved beyond drag threshold, it's active touch exploration
                if dist_sq >= self.drag_threshold_sq {
                    TouchResult::Explore { x, y }
                } else {
                    TouchResult::None
                }
            }
            ACTION_UP => {
                if !self.is_down {
                    return TouchResult::None;
                }
                self.is_down = false;

                let dx = x - self.down_x;
                let dy = y - self.down_y;
                let duration = time_ms - self.down_time;
                let dist_sq = dx * dx + dy * dy;

                // Check if flick swipe gesture (fast motion, large displacement)
                if duration < 250 && dist_sq > 2500.0 {
                    if dx.abs() > dy.abs() {
                        if dx > 0.0 {
                            return TouchResult::Flick(ScreenReaderCommand::ReviewNextCharacter);
                        } else {
                            return TouchResult::Flick(ScreenReaderCommand::ReviewPreviousCharacter);
                        }
                    } else if dy > 0.0 {
                        return TouchResult::Flick(ScreenReaderCommand::ReviewNextLine);
                    } else {
                        return TouchResult::Flick(ScreenReaderCommand::ReviewPreviousLine);
                    }
                }

                // If small movement, record as potential tap or first tap of a double-tap
                if dist_sq < self.drag_threshold_sq {
                    self.last_up_time = time_ms;
                    self.last_up_x = x;
                    self.last_up_y = y;
                    return TouchResult::Tap { x, y };
                }

                TouchResult::None
            }
            ACTION_CANCEL => {
                self.is_down = false;
                TouchResult::None
            }
            _ => TouchResult::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_double_tap_detection() {
        let mut sm = TouchStateMachine::new();

        // Tap 1
        assert_eq!(sm.process_touch(ACTION_DOWN, 100.0, 100.0, 1000), TouchResult::None);
        assert_eq!(sm.process_touch(ACTION_UP, 102.0, 101.0, 1050), TouchResult::Tap { x: 102.0, y: 101.0 });

        // Tap 2 within 150ms -> Double tap!
        let result = sm.process_touch(ACTION_DOWN, 105.0, 103.0, 1200);
        assert_eq!(result, TouchResult::DoubleTap { x: 105.0, y: 103.0 });
    }

    #[test]
    fn test_flick_swipe_right() {
        let mut sm = TouchStateMachine::new();

        sm.process_touch(ACTION_DOWN, 100.0, 100.0, 1000);
        let result = sm.process_touch(ACTION_UP, 300.0, 110.0, 1100);

        assert_eq!(result, TouchResult::Flick(ScreenReaderCommand::ReviewNextCharacter));
    }
}
