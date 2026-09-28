//! Safe Wrapper for IUIAutomationElement Reading Pre-Cached Properties.
//! Aligned with WINDOWS.md Section 3.4 and Section 3.5.

use bit_sr_core::node::{AccessibleNode, NodeId, PositionInfo, Rect};
use bit_sr_core::roles::Role;
use bit_sr_core::states::State;
use windows::Win32::UI::Accessibility::*;

pub fn uia_control_type_to_role(control_type_id: u32) -> Role {
    match control_type_id {
        50000 => Role::Button,
        50001 => Role::Calendar,
        50002 => Role::CheckBox,
        50003 => Role::ComboBox,
        50004 => Role::EditableText,
        50005 => Role::Link,
        50006 => Role::Graphic,
        50007 => Role::ListItem,
        50008 => Role::List,
        50009 => Role::PopupMenu,
        50010 => Role::MenuBar,
        50011 => Role::MenuItem,
        50012 => Role::ProgressBar,
        50013 => Role::RadioButton,
        50014 => Role::ScrollBar,
        50015 => Role::Slider,
        50016 => Role::SpinButton,
        50017 => Role::StatusBar,
        50018 => Role::TabControl,
        50019 => Role::Tab,
        50020 => Role::StaticText,
        50021 => Role::ToolBar,
        50022 => Role::ToolTip,
        50023 => Role::TreeView,
        50024 => Role::TreeViewItem,
        50025 => Role::Unknown,
        50026 => Role::Grouping,
        50027 => Role::Thumb,
        50028 => Role::DataGrid,
        50029 => Role::DataItem,
        50030 => Role::Document,
        50031 => Role::SplitButton,
        50032 => Role::Window,
        50033 => Role::Pane,
        50034 => Role::Header,
        50035 => Role::HeaderItem,
        50036 => Role::Table,
        50037 => Role::TitleBar,
        50038 => Role::Separator,
        _ => Role::Unknown,
    }
}

pub struct UiaElement {
    raw: IUIAutomationElement,
}

impl UiaElement {
    pub fn new(element: IUIAutomationElement) -> Self {
        Self { raw: element }
    }

    pub fn raw(&self) -> &IUIAutomationElement {
        &self.raw
    }

    /// Converts the cached properties of this UIA element into a unified AccessibleNode.
    /// Does not trigger out-of-process COM calls.
    pub fn to_accessible_node(&self) -> AccessibleNode {
        let name = unsafe {
            self.raw
                .CachedName()
                .ok()
                .map(|b| b.to_string())
                .filter(|s| !s.trim().is_empty())
        };

        let role = unsafe {
            self.raw
                .CachedControlType()
                .map(|id| uia_control_type_to_role(id.0 as u32))
                .unwrap_or(Role::Unknown)
        };

        let class_name = unsafe {
            self.raw
                .CachedClassName()
                .ok()
                .map(|b| b.to_string())
                .filter(|s| !s.trim().is_empty())
        };

        let automation_id = unsafe {
            self.raw
                .CachedAutomationId()
                .ok()
                .map(|b| b.to_string())
                .filter(|s| !s.trim().is_empty())
        };

        let process_id = unsafe { self.raw.CachedProcessId().ok().map(|p| p as u32) };

        let hwnd = unsafe { self.raw.CachedNativeWindowHandle().ok().map(|h| h.0 as usize) };

        let has_focus = unsafe { self.raw.CachedHasKeyboardFocus().unwrap_or_default().as_bool() };

        let mut states = State::empty();
        if has_focus {
            states |= State::FOCUSED;
        }

        let is_offscreen = unsafe {
            self.raw
                .GetCachedPropertyValue(UIA_IsOffscreenPropertyId)
                .ok()
                .and_then(|v| bool::try_from(&v).ok())
                .unwrap_or(false)
        };
        if is_offscreen {
            states |= State::OFFSCREEN;
        }

        let position_in_set = unsafe {
            self.raw
                .GetCachedPropertyValue(UIA_PositionInSetPropertyId)
                .ok()
                .and_then(|v| i32::try_from(&v).ok())
                .filter(|&val| val > 0)
        };

        let size_of_set = unsafe {
            self.raw
                .GetCachedPropertyValue(UIA_SizeOfSetPropertyId)
                .ok()
                .and_then(|v| i32::try_from(&v).ok())
                .filter(|&val| val > 0)
        };

        let level = unsafe {
            self.raw
                .GetCachedPropertyValue(UIA_LevelPropertyId)
                .ok()
                .and_then(|v| i32::try_from(&v).ok())
                .filter(|&val| val > 0)
        };

        let bounds = unsafe {
            self.raw.CachedBoundingRectangle().ok().map(|r| Rect {
                left: r.left as f64,
                top: r.top as f64,
                width: (r.right - r.left) as f64,
                height: (r.bottom - r.top) as f64,
            })
        };

        let id = NodeId(hwnd.unwrap_or(0) as u64);

        AccessibleNode {
            id,
            role,
            states,
            name,
            value: None,
            description: None,
            keyboard_shortcut: None,
            class_name,
            automation_id,
            bounds,
            position_info: PositionInfo {
                position_in_set,
                size_of_set,
                row_index: None,
                column_index: None,
                level,
            },
            process_id,
        }
    }
}
