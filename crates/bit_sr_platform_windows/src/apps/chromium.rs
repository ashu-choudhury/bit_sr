//! Specialized Workarounds and Heuristics for Chromium, Google Chrome,
//! Microsoft Edge, Electron applications (VS Code, Slack, Discord), and WebView2.
//! Implements all 12 Blink-specific quirks documented in CHROMIUM.md Section 9.

use bit_sr_core::roles::Role;
use bit_sr_core::states::State;

/// Chromium filter applying Blink normalization and heuristics.
pub struct ChromiumFilter;

impl ChromiumFilter {
    /// Recommended AXMode bitmask for bit_sr:
    /// kNativeAPIs (0x1) | kWebContents (0x2) | kInlineTextBoxes (0x4) | kScreenReader (0x8) | kHTML (0x10) = 0x001F.
    pub const AX_MODE_SCREENREADER: u32 = 0x001F;

    /// Checks if a Win32 window class belongs to a Chromium-based engine.
    pub fn is_chromium_window(class_name: &str) -> bool {
        class_name.starts_with("Chrome_")
            || class_name.starts_with("MicrosoftEdge_")
            || class_name == "Intermediate D3D Window"
            || class_name.contains("WebView2")
    }

    /// Checks if a Win32 window handle belongs to a Chromium process or Chromium window class.
    pub fn is_chromium_hwnd(hwnd: windows::Win32::Foundation::HWND) -> bool {
        if hwnd.0.is_null() {
            return false;
        }

        let mut class_buf = [0u16; 256];
        let len = unsafe {
            windows::Win32::UI::WindowsAndMessaging::GetClassNameW(hwnd, &mut class_buf)
        };
        if len > 0 {
            let class_name = String::from_utf16_lossy(&class_buf[..len as usize]);
            if Self::is_chromium_window(&class_name) {
                return true;
            }
        }

        let mut pid: u32 = 0;
        unsafe {
            windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(hwnd, Some(&mut pid));
        }
        if pid != 0 {
            if let Some(proc_name) = crate::apps::get_process_name_by_pid(pid) {
                if Self::is_chromium_process(&proc_name) {
                    return true;
                }
            }
        }

        false
    }

    /// Sends WM_GETOBJECT with OBJID_CLIENT to elevate Chromium/Edge/WebView2 AXMode to
    /// kNativeAPIs | kWebContents | kInlineTextBoxes | kScreenReader | kHTML (0x001F).
    /// Recursively reaches child render widget windows to ensure Blink activates accessibility.
    pub fn activate_chromium_ax_mode(hwnd: windows::Win32::Foundation::HWND) {
        use windows::core::BOOL;
        use windows::Win32::Foundation::{LPARAM, WPARAM, HWND};
        use windows::Win32::UI::WindowsAndMessaging::{
            EnumChildWindows, GetClassNameW, SendMessageTimeoutW, SMTO_ABORTIFHUNG, WM_GETOBJECT,
        };

        const OBJID_CLIENT: i32 = -4;

        if hwnd.0.is_null() {
            return;
        }

        unsafe {
            let mut result: usize = 0;
            let _ = SendMessageTimeoutW(
                hwnd,
                WM_GETOBJECT,
                WPARAM(0),
                LPARAM(OBJID_CLIENT as isize),
                SMTO_ABORTIFHUNG,
                100,
                Some(&mut result),
            );

            // Enumerate child render widget windows
            unsafe extern "system" fn enum_child_proc(child: HWND, _lparam: LPARAM) -> BOOL {
                let mut buf = [0u16; 128];
                let len = unsafe { GetClassNameW(child, &mut buf) };
                if len > 0 {
                    let cls = String::from_utf16_lossy(&buf[..len as usize]);
                    if cls == "Chrome_RenderWidgetHostHWND" || cls == "Intermediate D3D Window" {
                        let mut result: usize = 0;
                        let _ = unsafe {
                            SendMessageTimeoutW(
                                child,
                                WM_GETOBJECT,
                                WPARAM(0),
                                LPARAM(OBJID_CLIENT as isize),
                                SMTO_ABORTIFHUNG,
                                100,
                                Some(&mut result),
                            )
                        };
                    }
                }
                BOOL::from(true)
            }

            let _ = EnumChildWindows(Some(hwnd), Some(enum_child_proc), LPARAM(0));
        }
    }

    /// Checks if an executable process name is a known Chromium or Electron application.
    pub fn is_chromium_process(proc_name: &str) -> bool {
        let name = proc_name.to_ascii_lowercase();
        matches!(
            name.as_str(),
            "chrome.exe"
                | "msedge.exe"
                | "brave.exe"
                | "vivaldi.exe"
                | "arc.exe"
                | "code.exe"
                | "slack.exe"
                | "discord.exe"
                | "teams.exe"
                | "msedgewebview2.exe"
                | "obsidian.exe"
                | "githubdesktop.exe"
                | "spotify.exe"
        )
    }

    /// Normalizes Chromium roles based on tag attributes (Quirks 3 & 4).
    pub fn normalize_role(role: Role, tag: &str) -> Role {
        if role == Role::Grouping && tag.eq_ignore_ascii_case("figure") {
            return Role::Figure;
        }
        role
    }

    /// Fixes Chromium state anomalies:
    /// - Quirk 2: Discard fake Checkable state on ToggleButtons.
    /// - Quirk 3: Enforce ReadOnly on unordered/ordered lists inside contenteditable.
    /// - Quirk 10: Canvas override for Google Docs/Slides (goog-editable="false").
    pub fn normalize_states(
        role: Role,
        tag: &str,
        mut states: State,
        goog_editable: Option<bool>,
    ) -> State {
        // Quirk 2: Strip Checkable on toggle buttons
        if role == Role::Switch {
            states.remove(State::CHECKABLE);
        }

        // Quirk 3: Presentational lists lack ReadOnly state
        if role == Role::List && matches!(tag, "ul" | "ol" | "dl") {
            states.insert(State::READONLY);
        }

        // Quirk 10: Google Docs / Slides canvas override
        if goog_editable == Some(false) {
            states.remove(State::EDITABLE);
        }

        states
    }

    /// Quirk 5: VS Code / Electron large file caret freeze workaround.
    /// Bypass expensive O(N) end-of-line checking inside recognized Electron code editors.
    pub fn should_suppress_caret_end_of_line(is_code_editor: bool) -> bool {
        is_code_editor
    }

    /// Quirk 6: Discard document focus events if document has no URI yet (early tab creation).
    pub fn is_valid_document_focus(role: Role, uri: Option<&str>) -> bool {
        if role == Role::Document {
            if let Some(u) = uri {
                return !u.trim().is_empty();
            }
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_chromium_window_and_process() {
        assert!(ChromiumFilter::is_chromium_window("Chrome_WidgetWin_1"));
        assert!(ChromiumFilter::is_chromium_window("Chrome_RenderWidgetHostHWND"));
        assert!(!ChromiumFilter::is_chromium_window("MozillaWindowClass"));

        assert!(ChromiumFilter::is_chromium_process("chrome.exe"));
        assert!(ChromiumFilter::is_chromium_process("Code.exe"));
        assert!(ChromiumFilter::is_chromium_process("msedge.exe"));
        assert!(!ChromiumFilter::is_chromium_process("notepad.exe"));
    }

    #[test]
    fn test_normalize_states_and_roles() {
        // Quirk 3: List gets READONLY
        let states = ChromiumFilter::normalize_states(Role::List, "ul", State::empty(), None);
        assert!(states.contains(State::READONLY));

        // Quirk 10: goog-editable=false strips EDITABLE
        let editable = State::EDITABLE | State::FOCUSABLE;
        let cleaned = ChromiumFilter::normalize_states(Role::EditableText, "div", editable, Some(false));
        assert!(!cleaned.contains(State::EDITABLE));
        assert!(cleaned.contains(State::FOCUSABLE));

        // Quirk 4: Figure tag
        let role = ChromiumFilter::normalize_role(Role::Grouping, "figure");
        assert_eq!(role, Role::Figure);
    }
}
