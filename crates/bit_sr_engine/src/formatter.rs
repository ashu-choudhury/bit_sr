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
    /// Formats a full, human-friendly speech announcement for a focused accessible node using the default English locale.
    pub fn format_focus(node: &AccessibleNode, context: &mut FormatterContext) -> String {
        let loc = bit_sr_core::LocalizationManager::default();
        Self::format_focus_localized(node, context, &loc)
    }

    /// Formats a full, human-friendly speech announcement for a focused accessible node in the active locale.
    pub fn format_focus_localized(
        node: &AccessibleNode,
        _context: &mut FormatterContext,
        loc: &bit_sr_core::LocalizationManager,
    ) -> String {
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
            let role_name = node.role.display_name_localized(loc);
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
            parts.push(loc.t("state.checked").to_string());
        } else if node.states.contains(State::CHECKABLE) {
            parts.push(loc.t("state.not_checked").to_string());
        }

        // Expanded / Collapsed state:
        if node.states.contains(State::EXPANDED) {
            parts.push(loc.t("state.expanded").to_string());
        } else if node.states.contains(State::COLLAPSED) {
            parts.push(loc.t("state.collapsed").to_string());
        }

        // Selected state (only if selected and role is selectable item)
        if node.states.contains(State::SELECTED)
            && matches!(
                node.role,
                Role::ListItem | Role::Tab | Role::TreeViewItem | Role::TableCell | Role::DataItem
            )
        {
            parts.push(loc.t("state.selected").to_string());
        }

        // Unavailable / Disabled:
        if node.states.contains(State::UNAVAILABLE) {
            parts.push(loc.t("state.unavailable").to_string());
        }

        // Read-only:
        if node.states.contains(State::READONLY) && is_edit {
            parts.push(loc.t("state.readonly").to_string());
        }

        // Required:
        if node.states.contains(State::REQUIRED) {
            parts.push(loc.t("state.required").to_string());
        }

        // 6. Positional info ("3 of 12", "level 2")
        if let (Some(pos), Some(size)) = (node.position_info.position_in_set, node.position_info.size_of_set) {
            if size > 0 {
                let pos_str = pos.to_string();
                let size_str = size.to_string();
                parts.push(loc.t_args("format.pos_of_total", &[("pos", &pos_str), ("count", &size_str)]));
            }
        } else if let Some(item_info) = &node.collection_item_info {
            parts.push(format!("row {}, column {}", item_info.row_index + 1, item_info.column_index + 1));
        }

        if let Some(level) = node.position_info.level {
            let level_str = level.to_string();
            parts.push(loc.t_args("format.level", &[("level", &level_str)]));
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

    /// Formats state changes for active/focused nodes using default English locale.
    pub fn format_state_change(state: State, is_set: bool) -> Option<&'static str> {
        let loc = bit_sr_core::LocalizationManager::default();
        Self::format_state_change_localized(state, is_set, &loc)
    }

    /// Formats state changes for active/focused nodes in the active locale.
    pub fn format_state_change_localized(
        state: State,
        is_set: bool,
        loc: &bit_sr_core::LocalizationManager,
    ) -> Option<&'static str> {
        match state {
            State::CHECKED => {
                if is_set {
                    Some(loc.t("state.checked"))
                } else {
                    Some(loc.t("state.not_checked"))
                }
            }
            State::EXPANDED => {
                if is_set {
                    Some(loc.t("state.expanded"))
                } else {
                    Some(loc.t("state.collapsed"))
                }
            }
            State::COLLAPSED => {
                if is_set {
                    Some(loc.t("state.collapsed"))
                } else {
                    Some(loc.t("state.expanded"))
                }
            }
            State::SELECTED => {
                if is_set {
                    Some(loc.t("state.selected"))
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

    #[test]
    fn test_format_focus_localized() {
        let mut ctx = FormatterContext::default();
        let node = AccessibleNode {
            id: NodeId(10),
            name: Some("Aceptar".to_string()),
            role: Role::Button,
            ..Default::default()
        };

        let loc_es = bit_sr_core::LocalizationManager::new("es");
        let text_es = SpeechFormatter::format_focus_localized(&node, &mut ctx, &loc_es);
        assert_eq!(text_es, "Aceptar, botón");

        let loc_hi = bit_sr_core::LocalizationManager::new("hi");
        let text_hi = SpeechFormatter::format_focus_localized(&node, &mut ctx, &loc_hi);
        assert_eq!(text_hi, "Aceptar, बटन");

        // Test checkbox with states in Spanish
        let cb_node = AccessibleNode {
            id: NodeId(11),
            name: Some("Guardar contraseña".to_string()),
            role: Role::CheckBox,
            states: State::CHECKABLE | State::CHECKED,
            ..Default::default()
        };
        let cb_es = SpeechFormatter::format_focus_localized(&cb_node, &mut ctx, &loc_es);
        assert_eq!(cb_es, "Guardar contraseña, casilla de verificación, marcado");

        // Test list item with position in Hindi
        let list_node = AccessibleNode {
            id: NodeId(12),
            name: Some("दस्तावेज़".to_string()),
            role: Role::ListItem,
            states: State::SELECTED,
            position_info: PositionInfo {
                position_in_set: Some(2),
                size_of_set: Some(8),
                ..Default::default()
            },
            ..Default::default()
        };
        let list_hi = SpeechFormatter::format_focus_localized(&list_node, &mut ctx, &loc_hi);
        assert_eq!(list_hi, "दस्तावेज़, सूची आइटम, चयनित, 8 में से 2");
    }

    #[test]
    fn test_format_state_change_localized() {
        let loc_es = bit_sr_core::LocalizationManager::new("es");
        assert_eq!(
            SpeechFormatter::format_state_change_localized(State::CHECKED, true, &loc_es),
            Some("marcado")
        );
        assert_eq!(
            SpeechFormatter::format_state_change_localized(State::CHECKED, false, &loc_es),
            Some("no marcado")
        );

        let loc_de = bit_sr_core::LocalizationManager::new("de");
        assert_eq!(
            SpeechFormatter::format_state_change_localized(State::EXPANDED, true, &loc_de),
            Some("erweitert")
        );
        assert_eq!(
            SpeechFormatter::format_state_change_localized(State::EXPANDED, false, &loc_de),
            Some("reduziert")
        );
    }
}

