//! Unified Accessible State Bitflags.
//! Aligned with WINDOWS.md (Section 4.5) and LINUX.md (Section 3.2).

use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct State: u64 {
        const UNAVAILABLE      = 1 << 0;
        const FOCUSED          = 1 << 1;
        const SELECTED         = 1 << 2;
        const BUSY             = 1 << 3;
        const PRESSED          = 1 << 4;
        const CHECKED          = 1 << 5;
        const HALFCHECKED      = 1 << 6;
        const READONLY         = 1 << 7;
        const EXPANDED         = 1 << 8;
        const COLLAPSED        = 1 << 9;
        const INVISIBLE        = 1 << 10;
        const VISITED          = 1 << 11;
        const LINKED           = 1 << 12;
        const HASPOPUP         = 1 << 13;
        const PROTECTED        = 1 << 14;
        const REQUIRED         = 1 << 15;
        const DEFUNCT          = 1 << 16;
        const INVALID_ENTRY    = 1 << 17;
        const MODAL            = 1 << 18;
        const AUTOCOMPLETE     = 1 << 19;
        const MULTILINE        = 1 << 20;
        const ICONIFIED        = 1 << 21;
        const OFFSCREEN        = 1 << 22;
        const SELECTABLE       = 1 << 23;
        const FOCUSABLE        = 1 << 24;
        const CLICKABLE        = 1 << 25;
        const EDITABLE         = 1 << 26;
        const CHECKABLE        = 1 << 27;
        const HOTTRACKED       = 1 << 28;
        const INDETERMINATE    = 1 << 29;
        const MULTISELECTABLE  = 1 << 30;
        const ACTIVE           = 1 << 31;
        const ARMED            = 1 << 32;
        const EXPANDABLE       = 1 << 33;
        const SENSITIVE        = 1 << 34;
        const SHOWING          = 1 << 35;
        const SINGLE_LINE      = 1 << 36;
        const TRANSIENT        = 1 << 37;
        const VERTICAL         = 1 << 38;
        const HORIZONTAL       = 1 << 39;
        const VISIBLE          = 1 << 40;
    }
}

impl State {
    /// Formats active states into human-readable spoken phrases.
    pub fn to_speech_strings(&self) -> Vec<&'static str> {
        let mut list = Vec::new();
        if self.contains(Self::UNAVAILABLE) {
            list.push("unavailable");
        }
        if self.contains(Self::CHECKED) {
            list.push("checked");
        } else if self.contains(Self::HALFCHECKED) {
            list.push("half checked");
        }
        if self.contains(Self::PRESSED) {
            list.push("pressed");
        }
        if self.contains(Self::EXPANDED) {
            list.push("expanded");
        } else if self.contains(Self::COLLAPSED) {
            list.push("collapsed");
        }
        if self.contains(Self::SELECTED) {
            list.push("selected");
        }
        if self.contains(Self::BUSY) {
            list.push("busy");
        }
        if self.contains(Self::READONLY) {
            list.push("read only");
        }
        if self.contains(Self::REQUIRED) {
            list.push("required");
        }
        if self.contains(Self::INVALID_ENTRY) {
            list.push("invalid entry");
        }
        if self.contains(Self::HASPOPUP) {
            list.push("has popup");
        }
        list
    }

    /// Formats active states into localized spoken phrases using the given `LocalizationManager`.
    pub fn to_speech_strings_localized(&self, loc: &crate::i18n::LocalizationManager) -> Vec<&'static str> {
        let mut list = Vec::new();
        if self.contains(Self::UNAVAILABLE) {
            list.push(loc.t("state.unavailable"));
        }
        if self.contains(Self::CHECKED) {
            list.push(loc.t("state.checked"));
        } else if self.contains(Self::HALFCHECKED) {
            list.push(loc.t("state.halfchecked"));
        }
        if self.contains(Self::PRESSED) {
            list.push(loc.t("state.pressed"));
        }
        if self.contains(Self::EXPANDED) {
            list.push(loc.t("state.expanded"));
        } else if self.contains(Self::COLLAPSED) {
            list.push(loc.t("state.collapsed"));
        }
        if self.contains(Self::SELECTED) {
            list.push(loc.t("state.selected"));
        }
        if self.contains(Self::BUSY) {
            list.push(loc.t("state.busy"));
        }
        if self.contains(Self::READONLY) {
            list.push(loc.t("state.readonly"));
        }
        if self.contains(Self::REQUIRED) {
            list.push(loc.t("state.required"));
        }
        if self.contains(Self::INVALID_ENTRY) {
            list.push(loc.t("state.invalid_entry"));
        }
        if self.contains(Self::HASPOPUP) {
            list.push(loc.t("state.haspopup"));
        }
        list
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::LocalizationManager;

    #[test]
    fn test_state_to_speech_strings() {
        let state = State::CHECKED | State::UNAVAILABLE;
        let strings = state.to_speech_strings();
        assert_eq!(strings, vec!["unavailable", "checked"]);

        let expanded_selected = State::EXPANDED | State::SELECTED;
        assert_eq!(expanded_selected.to_speech_strings(), vec!["expanded", "selected"]);
    }

    #[test]
    fn test_state_localized_speech_strings() {
        let state = State::CHECKED | State::UNAVAILABLE;

        let loc_es = LocalizationManager::new("es");
        assert_eq!(state.to_speech_strings_localized(&loc_es), vec!["no disponible", "marcado"]);

        let loc_hi = LocalizationManager::new("hi");
        assert_eq!(state.to_speech_strings_localized(&loc_hi), vec!["अनुपलब्ध", "चेक किया गया"]);
    }
}
