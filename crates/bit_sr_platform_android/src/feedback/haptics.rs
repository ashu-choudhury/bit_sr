//! Haptic Feedback Subsystem for Android.
//! Integrates with `android.os.Vibrator` / `VibratorManager` via JNI.

/// Haptic vibration effects for Android screen reader interactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HapticEffect {
    /// Light subtle tick on touching any UI element.
    Tick,
    /// Confirmation click when activating an element.
    Click,
    /// Heavy vibration when reaching a boundary or trigger error.
    HeavyClick,
    /// Double pulse for mode changes or global menus.
    DoublePulse,
}

impl HapticEffect {
    /// Integer code matching Android `VibrationEffect.EFFECT_*` predefined constants.
    pub fn aosp_effect_id(&self) -> i32 {
        match self {
            Self::Tick => 2,        // EFFECT_TICK
            Self::Click => 0,       // EFFECT_CLICK
            Self::HeavyClick => 5,  // EFFECT_HEAVY_CLICK
            Self::DoublePulse => 1, // EFFECT_DOUBLE_CLICK
        }
    }
}
