//! Android View Class Name to unified `bit_sr_core::Role` and `State` mapper.

use bit_sr_core::roles::Role;
use bit_sr_core::states::State;

/// State bitflags passed from Android service via JNI.
pub const STATE_FOCUSED: i64 = 1 << 0;
pub const STATE_ACCESSIBILITY_FOCUSED: i64 = 1 << 1;
pub const STATE_CHECKED: i64 = 1 << 2;
pub const STATE_SELECTED: i64 = 1 << 3;
pub const STATE_DISABLED: i64 = 1 << 4;

/// Maps an Android View class name (e.g. "android.widget.Button") to a unified `Role`.
pub fn map_class_name_to_role(class_name: &str) -> Role {
    match class_name {
        "android.widget.Button" => Role::Button,
        "android.widget.EditText" => Role::EditableText,
        "android.widget.CheckBox" => Role::CheckBox,
        "android.widget.RadioButton" => Role::RadioButton,
        "android.widget.Switch" | "androidx.appcompat.widget.SwitchCompat" => Role::Switch,
        "android.widget.SeekBar" => Role::Slider,
        "android.widget.ProgressBar" => Role::ProgressBar,
        "android.widget.TextView" => Role::StaticText,
        "android.widget.ImageView" | "android.widget.ImageButton" => Role::Graphic,
        "android.widget.ListView" => Role::List,
        "android.widget.GridView" => Role::DataGrid,
        "android.widget.ScrollView" => Role::Pane,
        "android.webkit.WebView" => Role::Document,
        "android.widget.TabWidget" => Role::TabControl,
        "android.widget.Spinner" => Role::ComboBox,
        _ => {
            // Heuristic fallbacks for custom subclasses
            if class_name.ends_with("Button") {
                Role::Button
            } else if class_name.ends_with("EditText") {
                Role::EditableText
            } else if class_name.ends_with("CheckBox") {
                Role::CheckBox
            } else if class_name.ends_with("Switch") {
                Role::Switch
            } else if class_name.ends_with("TextView") {
                Role::StaticText
            } else {
                Role::Unknown
            }
        }
    }
}

/// Unpacks a 64-bit integer bitmask of Android states into `bit_sr_core::State`.
pub fn unpack_states(state_bits: i64) -> State {
    let mut states = State::empty();

    if (state_bits & STATE_FOCUSED) != 0 {
        states.insert(State::FOCUSED);
    }
    if (state_bits & STATE_ACCESSIBILITY_FOCUSED) != 0 {
        states.insert(State::ACTIVE);
    }
    if (state_bits & STATE_CHECKED) != 0 {
        states.insert(State::CHECKED);
    }
    if (state_bits & STATE_SELECTED) != 0 {
        states.insert(State::SELECTED);
    }
    if (state_bits & STATE_DISABLED) != 0 {
        states.insert(State::UNAVAILABLE);
    }

    states
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_mappings() {
        assert_eq!(map_class_name_to_role("android.widget.Button"), Role::Button);
        assert_eq!(map_class_name_to_role("android.widget.EditText"), Role::EditableText);
        assert_eq!(map_class_name_to_role("android.widget.Switch"), Role::Switch);
        assert_eq!(map_class_name_to_role("com.google.android.material.button.MaterialButton"), Role::Button);
    }

    #[test]
    fn test_state_unpacking() {
        let bits = STATE_FOCUSED | STATE_CHECKED;
        let states = unpack_states(bits);
        assert!(states.contains(State::FOCUSED));
        assert!(states.contains(State::CHECKED));
        assert!(!states.contains(State::UNAVAILABLE));
    }
}
