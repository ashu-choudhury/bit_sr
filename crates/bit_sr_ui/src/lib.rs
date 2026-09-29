//! Accessible Native GUI Subsystem for `bit_sr`.
//!
//! Provides the accessible settings dashboard and AccessKit integration powered by Slint.
//! 100% platform-agnostic with zero OS-specific API calls.

slint::include_modules!();

pub mod controller;
pub mod settings;

pub use controller::{UiCommand, UiEvent, UiHandle};
pub use settings::UiSettings;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_settings_defaults() {
        let settings = UiSettings::default();
        assert_eq!(settings.speech_rate, 50.0);
        assert_eq!(settings.speech_volume, 80.0);
        assert_eq!(settings.speech_pitch, 50.0);
    }
}
