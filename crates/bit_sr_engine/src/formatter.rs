//! Speech Formatter: Converts accessible nodes and tree context into human-readable speech phrases.

use bit_sr_core::node::AccessibleNode;
use bit_sr_core::roles::Role;
use bit_sr_core::states::State;

/// Contextual tracking state for speech formatting to avoid redundant announcements.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FormatterContext {
    pub last_window_title: Option<String>,
}

pub struct SpeechFormatter;

impl SpeechFormatter {
    /// Formats a full, human-friendly speech announcement for a focused accessible node.
    pub fn format_focus(node: &AccessibleNode, _context: &mut FormatterContext) -> String {
        let mut parts = Vec::new();

        // 1. Name or Label
        let name = node.name.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty());

        // 2. Value (for edits, sliders, combos)
        let value = node.value.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty());

        let is_edit = matches!(node.role, Role::EditableText | Role::Terminal | Role::Document);

        if let Some(n) = name {
            parts.push(n.to_string());
        }

        // 3. Role
        let should_announce_role = match node.role {
            Role::Unknown | Role::Pane => false,
            Role::Window => parts.is_empty(), // only announce window if no other name
            _ => true,
        };

        if should_announce_role {
            let role_name = match node.role {
                Role::ListItem => "list item",
                Role::CheckBox => "check box",
                Role::RadioButton => "radio button",
                Role::EditableText => "edit",
                Role::ProgressBar => "progress bar",
                Role::TreeViewItem => "tree item",
                Role::TableCell => "cell",
                _ => node.role.display_name(),
            };
            parts.push(role_name.to_string());
        }

        // 4. Value / text content
        if is_edit {
            if let Some(v) = value {
                if Some(v) != name {
                    parts.push(v.to_string());
                }
            }
        } else if let Some(v) = value {
            parts.push(v.to_string());
        } else if let Some(range) = &node.range_info {
            parts.push(format!("{:.0}%", range.current));
        }

        // 5. States
        // Checkable state:
        if node.states.contains(State::CHECKED) {
            parts.push("checked".to_string());
        } else if node.states.contains(State::CHECKABLE) {
            parts.push("not checked".to_string());
        }

        // Expanded / Collapsed state:
        if node.states.contains(State::EXPANDED) {
            parts.push("expanded".to_string());
        } else if node.states.contains(State::COLLAPSED) {
            parts.push("collapsed".to_string());
        }

        // Selected state (only if selected and role is selectable item)
        if node.states.contains(State::SELECTED)
            && matches!(
                node.role,
                Role::ListItem | Role::Tab | Role::TreeViewItem | Role::TableCell | Role::DataItem
            )
        {
            parts.push("selected".to_string());
        }

        // Unavailable / Disabled:
        if node.states.contains(State::UNAVAILABLE) {
            parts.push("unavailable".to_string());
        }

        // Read-only:
        if node.states.contains(State::READONLY) && is_edit {
            parts.push("read only".to_string());
        }

        // Required:
        if node.states.contains(State::REQUIRED) {
            parts.push("required".to_string());
        }

        // 6. Positional info ("3 of 12", "level 2")
        if let (Some(pos), Some(size)) = (node.position_info.position_in_set, node.position_info.size_of_set) {
            if size > 0 {
                parts.push(format!("{} of {}", pos, size));
            }
        } else if let Some(item_info) = &node.collection_item_info {
            parts.push(format!("row {}, column {}", item_info.row_index + 1, item_info.column_index + 1));
        }

        if let Some(level) = node.position_info.level {
            parts.push(format!("level {}", level));
        }

        // 7. Keyboard Shortcut
        if let Some(shortcut) = node.keyboard_shortcut.as_deref().filter(|s| !s.trim().is_empty()) {
            parts.push(shortcut.trim().to_string());
        }

        // 8. Description / Hint
        if let Some(desc) = node.description.as_deref().filter(|s| !s.trim().is_empty()) {
            parts.push(desc.trim().to_string());
        }

        parts.join(", ")
    }

    /// Formats state changes for active/focused nodes.
    pub fn format_state_change(state: State, is_set: bool) -> Option<&'static str> {
        match state {
            State::CHECKED => {
                if is_set {
                    Some("checked")
                } else {
                    Some("not checked")
                }
            }
            State::EXPANDED => {
                if is_set {
                    Some("expanded")
                } else {
                    Some("collapsed")
                }
            }
            State::COLLAPSED => {
                if is_set {
                    Some("collapsed")
                } else {
                    Some("expanded")
                }
            }
            State::SELECTED => {
                if is_set {
                    Some("selected")
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bit_sr_core::node::{NodeId, PositionInfo, RangeInfo};

    #[test]
    fn test_format_button() {
        let mut ctx = FormatterContext::default();
        let node = AccessibleNode {
            id: NodeId(1),
            name: Some("Save".to_string()),
            role: Role::Button,
            keyboard_shortcut: Some("Ctrl+S".to_string()),
            ..Default::default()
        };
        let text = SpeechFormatter::format_focus(&node, &mut ctx);
        assert_eq!(text, "Save, button, Ctrl+S");
    }

    #[test]
    fn test_format_checkbox() {
        let mut ctx = FormatterContext::default();
        let mut node = AccessibleNode {
            id: NodeId(2),
            name: Some("Auto-save".to_string()),
            role: Role::CheckBox,
            states: State::CHECKABLE | State::CHECKED,
            ..Default::default()
        };
        let text = SpeechFormatter::format_focus(&node, &mut ctx);
        assert_eq!(text, "Auto-save, check box, checked");

        node.states = State::CHECKABLE;
        let text_unchecked = SpeechFormatter::format_focus(&node, &mut ctx);
        assert_eq!(text_unchecked, "Auto-save, check box, not checked");
    }

    #[test]
    fn test_format_list_item_with_position() {
        let mut ctx = FormatterContext::default();
        let node = AccessibleNode {
            id: NodeId(3),
            name: Some("Documents".to_string()),
            role: Role::ListItem,
            states: State::SELECTED,
            position_info: PositionInfo {
                position_in_set: Some(3),
                size_of_set: Some(15),
                ..Default::default()
            },
            ..Default::default()
        };
        let text = SpeechFormatter::format_focus(&node, &mut ctx);
        assert_eq!(text, "Documents, list item, selected, 3 of 15");
    }

    #[test]
    fn test_format_slider() {
        let mut ctx = FormatterContext::default();
        let node = AccessibleNode {
            id: NodeId(4),
            name: Some("Volume".to_string()),
            role: Role::Slider,
            range_info: Some(RangeInfo {
                min: 0.0,
                max: 100.0,
                current: 75.0,
                step: 5.0,
            }),
            ..Default::default()
        };
        let text = SpeechFormatter::format_focus(&node, &mut ctx);
        assert_eq!(text, "Volume, slider, 75%");
    }
}
