//! Virtual Soft Keyboard (IME) window layering and exploration heuristics.
//! Handles Gboard, Samsung Keyboard, and OEM input methods.

use bit_sr_core::node::Rect;

/// Soft Keyboard (IME) filter.
pub struct SoftKeyboardFilter;

impl SoftKeyboardFilter {
    /// Determines if a package belongs to a virtual soft keyboard.
    pub fn is_keyboard_package(pkg: &str) -> bool {
        pkg == "com.google.android.inputmethod.latin"
            || pkg == "com.sec.android.inputmethod"
            || pkg.contains("keyboard")
            || pkg.contains("inputmethod")
    }

    /// Evaluates if an on-screen coordinate is in the keyboard zone.
    pub fn is_coordinate_in_keyboard_bounds(x: f64, y: f64, ime_bounds: Option<&Rect>) -> bool {
        if let Some(bounds) = ime_bounds {
            bounds.contains_point(x, y)
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyboard_detection() {
        assert!(SoftKeyboardFilter::is_keyboard_package("com.google.android.inputmethod.latin"));
        assert!(SoftKeyboardFilter::is_keyboard_package("com.sec.android.inputmethod"));
        assert!(!SoftKeyboardFilter::is_keyboard_package("com.android.chrome"));
    }
}
