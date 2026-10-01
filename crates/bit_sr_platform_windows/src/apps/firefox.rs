//! Specialized Workarounds and Heuristics for Mozilla Firefox, Firefox ESR,
//! Tor Browser, LibreWolf, and Thunderbird.
//! Implements all Gecko-specific heuristics documented in FIREFOX.md Section 7.

use bit_sr_core::roles::Role;
use bit_sr_core::states::State;
use std::sync::Mutex;
use std::time::Instant;

/// Gecko heuristics normalizer for Windows.
pub struct FirefoxFilter {
    last_tab_switch: Mutex<Option<Instant>>,
}

impl Default for FirefoxFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl FirefoxFilter {
    pub fn new() -> Self {
        Self {
            last_tab_switch: Mutex::new(None),
        }
    }

    /// Checks if a Win32 window class belongs to Mozilla Gecko.
    pub fn is_gecko_window(class_name: &str) -> bool {
        matches!(
            class_name,
            "MozillaWindowClass"
                | "MozillaContentWindowClass"
                | "MozillaDialogClass"
                | "GeckoPluginWindow"
        )
    }

    /// Checks if an executable process name is a known Gecko application.
    pub fn is_gecko_process(proc_name: &str) -> bool {
        let name = proc_name.to_ascii_lowercase();
        matches!(
            name.as_str(),
            "firefox.exe"
                | "thunderbird.exe"
                | "tor.exe"
                | "librewolf.exe"
                | "waterfox.exe"
        )
    }

    /// Quirk 1: STATE_SYSTEM_MARQUEED (0x00008000) signals checkable in certain Gecko controls.
    pub fn normalize_states(raw_msaa_states: u32, mut states: State) -> State {
        const STATE_SYSTEM_MARQUEED: u32 = 0x0000_8000;
        if (raw_msaa_states & STATE_SYSTEM_MARQUEED) != 0 {
            states.insert(State::CHECKABLE);
        }
        states
    }

    /// Quirk 3: Determines if a focus ancestor should be suppressed to prevent chatter.
    /// Redundant table rows are excluded because cells already report row and column coordinates.
    pub fn should_suppress_focus_ancestor(role: Role) -> bool {
        role == Role::TableRow
    }

    /// Quirk 4: Focus Redirection in Multi-Tab Sessions (15ms coalescing window).
    pub fn register_tab_switch(&self) {
        let mut guard = self.last_tab_switch.lock().unwrap();
        *guard = Some(Instant::now());
    }

    /// Checks whether an intermediate window focus event should be coalesced during tab switch.
    pub fn is_tab_switch_coalesced(&self, role: Role) -> bool {
        let guard = self.last_tab_switch.lock().unwrap();
        if let Some(t) = *guard {
            if t.elapsed().as_millis() < 15 && role == Role::Window {
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
    fn test_is_gecko_window_and_process() {
        assert!(FirefoxFilter::is_gecko_window("MozillaWindowClass"));
        assert!(FirefoxFilter::is_gecko_window("MozillaContentWindowClass"));
        assert!(!FirefoxFilter::is_gecko_window("Chrome_WidgetWin_1"));

        assert!(FirefoxFilter::is_gecko_process("firefox.exe"));
        assert!(FirefoxFilter::is_gecko_process("tor.exe"));
        assert!(FirefoxFilter::is_gecko_process("Thunderbird.exe"));
        assert!(!FirefoxFilter::is_gecko_process("chrome.exe"));
    }

    #[test]
    fn test_gecko_state_normalization() {
        let states = FirefoxFilter::normalize_states(0x0000_8000, State::empty());
        assert!(states.contains(State::CHECKABLE));
    }

    #[test]
    fn test_gecko_table_row_suppression() {
        assert!(FirefoxFilter::should_suppress_focus_ancestor(Role::TableRow));
        assert!(!FirefoxFilter::should_suppress_focus_ancestor(Role::TableCell));
        assert!(!FirefoxFilter::should_suppress_focus_ancestor(Role::Table));
    }
}
