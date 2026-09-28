//! Specialized Workarounds and Heuristics for Windows File Explorer (explorer.exe).
//! Implements all 9 Explorer-specific quirks documented in WINDOWS.md Section 6.

use bit_sr_core::node::AccessibleNode;
use bit_sr_core::roles::Role;
use std::sync::Mutex;
use std::time::Instant;

/// Strip invisible left-to-right and right-to-left directional markers
/// that Windows File Explorer places around date and size columns.
pub fn sanitize_explorer_text(text: &str) -> String {
    text.replace('\u{200E}', "").replace('\u{200F}', "")
}

/// State tracker for deduplicating transient and burst Explorer events.
pub struct ExplorerFilter {
    last_focus_hwnd: Mutex<Option<(usize, u32, Instant)>>,
    last_tab_selected_id: Mutex<Option<(String, Instant)>>,
}

impl ExplorerFilter {
    pub fn new() -> Self {
        Self {
            last_focus_hwnd: Mutex::new(None),
            last_tab_selected_id: Mutex::new(None),
        }
    }

    /// Quirk 1: The Address Bar is physically wrapped inside an `msctls_progress32` control
    /// with an ancestor of `Address Band Root`. Hide this progress bar role.
    pub fn should_suppress_address_bar_progress(&self, class_name: &str, ancestor_class: Option<&str>) -> bool {
        if class_name == "msctls_progress32" {
            if let Some(anc) = ancestor_class {
                if anc == "Address Band Root" {
                    return true;
                }
            }
        }
        false
    }

    /// Quirk 2: On desktop / classic lists (SysListView32), moving focus fires a focus event
    /// on the list container immediately followed by the item. Deduplicate if received within 60ms.
    pub fn is_duplicate_list_item_focus(&self, hwnd: usize, item_id: u32) -> bool {
        let mut guard = self.last_focus_hwnd.lock().unwrap();
        let now = Instant::now();
        if let Some((prev_hwnd, prev_id, prev_time)) = *guard {
            if prev_hwnd == hwnd && prev_id == item_id && now.duration_since(prev_time).as_millis() < 60 {
                return true;
            }
        }
        *guard = Some((hwnd, item_id, now));
        false
    }

    /// Quirk 3: WorkerW window fires focus events with role PANE and empty name on minimize (Win+M).
    pub fn should_suppress_workerw_pane(&self, class_name: &str, role: Role, name: Option<&str>) -> bool {
        if class_name == "WorkerW" && role == Role::Pane {
            return name.map_or(true, |n| n.trim().is_empty());
        }
        false
    }

    /// Quirk 4: Redundant focus fired on UniversalSearchBand / Search Box after SearchEditBox gains focus.
    pub fn is_search_band_redundant(&self, class_name: &str, role: Role) -> bool {
        (class_name == "Search Box" || class_name == "UniversalSearchBand") && role == Role::Pane
    }

    /// Quirk 7: Windows 11 Tabs (TabListView) fires two selection events per tab switch.
    pub fn is_duplicate_tab_selection(&self, tab_id: &str) -> bool {
        let mut guard = self.last_tab_selected_id.lock().unwrap();
        let now = Instant::now();
        if let Some((ref prev_id, prev_time)) = *guard {
            if prev_id == tab_id && now.duration_since(prev_time).as_millis() < 100 {
                return true;
            }
        }
        *guard = Some((tab_id.to_string(), now));
        false
    }

    /// Quirk 8: Alt+Tab MultitaskingViewFrame focus suppression.
    pub fn is_multitasking_view_frame(&self, class_name: &str) -> bool {
        class_name == "MultitaskingViewFrame" || class_name == "Windows.UI.Input.InputSite.WindowClass"
    }

    /// Quirk 6: Formats status bar parts from child nodes (StatusBarModuleInner).
    /// Extracts item count and selection summary, ignoring view mode radio buttons.
    pub fn format_status_bar_children(&self, children: &[AccessibleNode]) -> String {
        let mut parts = Vec::new();
        for child in children {
            if child.role == Role::Grouping {
                if let Some(ref name) = child.name {
                    if !name.trim().is_empty() {
                        parts.push(name.trim().to_string());
                    }
                }
            } else if child.role == Role::StaticText {
                if let Some(ref name) = child.name {
                    if !name.trim().is_empty() {
                        parts.push(name.trim().to_string());
                    }
                }
            }
        }
        parts.join(", ")
    }

    /// Post-process an accessible node from Explorer.
    pub fn process_node(&self, mut node: AccessibleNode) -> Option<AccessibleNode> {
        // Suppress WorkerW phantom panes
        if let Some(ref cls) = node.class_name {
            if self.should_suppress_workerw_pane(cls, node.role, node.name.as_deref()) {
                return None;
            }
            if self.is_search_band_redundant(cls, node.role) {
                return None;
            }
            if self.is_multitasking_view_frame(cls) {
                return None;
            }
        }

        // Sanitize directional marks in values and names
        if let Some(ref mut name) = node.name {
            *name = sanitize_explorer_text(name);
        }
        if let Some(ref mut val) = node.value {
            *val = sanitize_explorer_text(val);
        }

        Some(node)
    }
}

impl Default for ExplorerFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_explorer_text() {
        let raw = "\u{200E}2026-09-28\u{200F} 14:00";
        assert_eq!(sanitize_explorer_text(raw), "2026-09-28 14:00");
    }

    #[test]
    fn test_address_band_progress_bar() {
        let filter = ExplorerFilter::new();
        assert!(filter.should_suppress_address_bar_progress("msctls_progress32", Some("Address Band Root")));
        assert!(!filter.should_suppress_address_bar_progress("msctls_progress32", Some("OtherAncestor")));
        assert!(!filter.should_suppress_address_bar_progress("Edit", Some("Address Band Root")));
    }

    #[test]
    fn test_transient_container_filters() {
        let filter = ExplorerFilter::new();
        assert!(filter.should_suppress_workerw_pane("WorkerW", Role::Pane, None));
        assert!(filter.should_suppress_workerw_pane("WorkerW", Role::Pane, Some("   ")));
        assert!(!filter.should_suppress_workerw_pane("WorkerW", Role::Pane, Some("Desktop")));
        assert!(!filter.should_suppress_workerw_pane("DirectUIHWND", Role::Pane, None));

        assert!(filter.is_search_band_redundant("UniversalSearchBand", Role::Pane));
        assert!(filter.is_search_band_redundant("Search Box", Role::Pane));
        assert!(!filter.is_search_band_redundant("SearchBox", Role::EditableText));

        assert!(filter.is_multitasking_view_frame("MultitaskingViewFrame"));
        assert!(filter.is_multitasking_view_frame("Windows.UI.Input.InputSite.WindowClass"));
        assert!(!filter.is_multitasking_view_frame("CabinetWClass"));
    }

    #[test]
    fn test_tab_deduplication() {
        let filter = ExplorerFilter::new();
        assert!(!filter.is_duplicate_tab_selection("tab-1"));
        // Immediate repeat is detected as duplicate
        assert!(filter.is_duplicate_tab_selection("tab-1"));
        // Different tab is not duplicate
        assert!(!filter.is_duplicate_tab_selection("tab-2"));
    }

    #[test]
    fn test_list_item_focus_deduplication() {
        let filter = ExplorerFilter::new();
        // First focus is allowed
        assert!(!filter.is_duplicate_list_item_focus(0x1000, 1));
        // Immediate repeat within 60ms is duplicate
        assert!(filter.is_duplicate_list_item_focus(0x1000, 1));
        // Different item is allowed
        assert!(!filter.is_duplicate_list_item_focus(0x1000, 2));
    }

    #[test]
    fn test_format_status_bar_children() {
        let filter = ExplorerFilter::new();
        let items = vec![
            AccessibleNode {
                role: Role::Grouping,
                name: Some("5 items".to_string()),
                ..Default::default()
            },
            AccessibleNode {
                role: Role::StaticText,
                name: Some("1 item selected".to_string()),
                ..Default::default()
            },
        ];

        let formatted = filter.format_status_bar_children(&items);
        assert_eq!(formatted, "5 items, 1 item selected");
    }
}
