//! Safe Wrapper for IUIAutomationElement Reading Pre-Cached Properties.
//! Aligned with WINDOWS.md Section 3.4 and Section 3.5.

use bit_sr_core::node::{AccessibleNode, NodeId, PositionInfo, Rect};
use bit_sr_core::roles::Role;
use bit_sr_core::states::State;
use windows::core::Interface;
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

unsafe fn variant_to_string(v: &windows::Win32::System::Variant::VARIANT) -> Option<String> {
    unsafe {
        if v.Anonymous.Anonymous.vt == windows::Win32::System::Variant::VT_BSTR {
            let bstr = &v.Anonymous.Anonymous.Anonymous.bstrVal;
            let s = bstr.to_string();
            if !s.trim().is_empty() {
                Some(s)
            } else {
                None
            }
        } else {
            None
        }
    }
}

unsafe fn extract_runtime_id_hash(sa: *mut windows::Win32::System::Com::SAFEARRAY) -> Option<u64> {
    if sa.is_null() {
        return None;
    }
    unsafe {
        if (*sa).cDims < 1 {
            let _ = windows::Win32::System::Ole::SafeArrayDestroy(sa);
            return None;
        }
        let mut p_data: *mut std::ffi::c_void = std::ptr::null_mut();
        if windows::Win32::System::Ole::SafeArrayAccessData(sa, &mut p_data).is_ok() {
            let count = (*sa).rgsabound[0].cElements as usize;
            if count > 0 && !p_data.is_null() {
                let slice = std::slice::from_raw_parts(p_data as *const i32, count);
                let mut hash: u64 = 0xcbf29ce484222325;
                for &num in slice {
                    hash ^= num as u64;
                    hash = hash.wrapping_mul(0x100000001b3);
                }
                let _ = windows::Win32::System::Ole::SafeArrayUnaccessData(sa);
                let _ = windows::Win32::System::Ole::SafeArrayDestroy(sa);
                return Some(hash);
            }
            let _ = windows::Win32::System::Ole::SafeArrayUnaccessData(sa);
        }
        let _ = windows::Win32::System::Ole::SafeArrayDestroy(sa);
        None
    }
}

/// Maps UIA Control Type and ARIA Role (for web/Electron content) to a unified Role.
pub fn map_control_type_and_aria_role(control_type_id: u32, aria_role: Option<&str>) -> Role {
    if let Some(aria) = aria_role {
        let aria_clean = aria.trim().to_ascii_lowercase();
        match aria_clean.as_str() {
            "heading" => return Role::Heading,
            "button" => return Role::Button,
            "link" => return Role::Link,
            "article" => return Role::Article,
            "blockquote" => return Role::BlockQuote,
            "figure" => return Role::Figure,
            "checkbox" => return Role::CheckBox,
            "radio" => return Role::RadioButton,
            "switch" => return Role::Switch,
            "textbox" | "searchbox" => return Role::EditableText,
            "combobox" => return Role::ComboBox,
            "list" => return Role::List,
            "listitem" => return Role::ListItem,
            "table" => return Role::Table,
            "grid" => return Role::DataGrid,
            "row" => return Role::TableRow,
            "cell" | "gridcell" => return Role::TableCell,
            "columnheader" => return Role::TableColumnHeader,
            "rowheader" => return Role::TableRowHeader,
            "progressbar" => return Role::ProgressBar,
            "slider" => return Role::Slider,
            "spinbutton" => return Role::SpinButton,
            "tab" => return Role::Tab,
            "tablist" => return Role::TabControl,
            "tree" => return Role::TreeView,
            "treeitem" => return Role::TreeViewItem,
            "dialog" => return Role::Dialog,
            "alert" | "status" | "log" => return Role::Alert,
            "banner" | "navigation" | "main" | "contentinfo" | "search" | "complementary"
            | "region" => return Role::Landmark,
            "document" => return Role::Document,
            "form" => return Role::Form,
            "separator" => return Role::Separator,
            "menu" => return Role::PopupMenu,
            "menubar" => return Role::MenuBar,
            "menuitem" => return Role::MenuItem,
            "menuitemcheckbox" => return Role::CheckBox,
            "menuitemradio" => return Role::RadioButton,
            "tooltip" => return Role::ToolTip,
            "img" | "image" | "graphics-symbol" | "graphics-document" => return Role::Graphic,
            "paragraph" => return Role::Paragraph,
            "math" => return Role::Math,
            "code" | "caption" | "term" | "definition" => return Role::StaticText,
            "doc-footnote" | "doc-endnote" => return Role::Link,
            "option" => return Role::ListItem,
            "meter" => return Role::ProgressBar,
            "radiogroup" | "group" => return Role::Grouping,
            "toolbar" => return Role::ToolBar,
            _ => {}
        }
    }

    if control_type_id == 50034 {
        return Role::Heading;
    }

    uia_control_type_to_role(control_type_id)
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

    /// Queries the NativeWindowHandle (HWND) for this element.
    pub fn hwnd(&self) -> Option<usize> {
        unsafe {
            self.raw
                .CachedNativeWindowHandle()
                .or_else(|_| self.raw.CurrentNativeWindowHandle())
                .ok()
                .map(|h| h.0 as usize)
                .filter(|&h| h != 0)
        }
    }

    /// Converts this UIA element into a unified AccessibleNode.
    /// Gracefully falls back from Cached to Current properties to guarantee out-of-process
    /// and webview element retrieval without failing on un-cached event targets.
    pub fn to_accessible_node(&self) -> AccessibleNode {
        let mut name = unsafe {
            self.raw
                .CachedName()
                .or_else(|_| self.raw.CurrentName())
                .ok()
                .map(|b| b.to_string())
                .filter(|s| !s.trim().is_empty())
        };

        if name.is_none() {
            if let Ok(tp) = unsafe { self.raw.GetCurrentPatternAs::<IUIAutomationTextPattern>(UIA_TextPatternId) } {
                if let Ok(range) = unsafe { tp.DocumentRange() } {
                    if let Ok(text) = unsafe { range.GetText(256) } {
                        let s = text.to_string();
                        if !s.trim().is_empty() {
                            name = Some(s);
                        }
                    }
                }
            }
        }

        let aria_role = unsafe {
            self.raw
                .GetCachedPropertyValue(UIA_AriaRolePropertyId)
                .or_else(|_| self.raw.GetCurrentPropertyValue(UIA_AriaRolePropertyId))
                .ok()
                .and_then(|v| variant_to_string(&v))
                .filter(|s| !s.trim().is_empty())
        };

        let control_type_id = unsafe {
            self.raw
                .CachedControlType()
                .or_else(|_| self.raw.CurrentControlType())
                .map(|id| id.0 as u32)
                .unwrap_or(0)
        };

        let mut role = map_control_type_and_aria_role(control_type_id, aria_role.as_deref());

        let class_name = unsafe {
            self.raw
                .CachedClassName()
                .or_else(|_| self.raw.CurrentClassName())
                .ok()
                .map(|b| b.to_string())
                .filter(|s| !s.trim().is_empty())
        };

        let automation_id = unsafe {
            self.raw
                .CachedAutomationId()
                .or_else(|_| self.raw.CurrentAutomationId())
                .ok()
                .map(|b| b.to_string())
                .filter(|s| !s.trim().is_empty())
        };

        let process_id = unsafe {
            self.raw
                .CachedProcessId()
                .or_else(|_| self.raw.CurrentProcessId())
                .ok()
                .map(|p| p as u32)
        };

        let hwnd = unsafe {
            self.raw
                .CachedNativeWindowHandle()
                .or_else(|_| self.raw.CurrentNativeWindowHandle())
                .ok()
                .map(|h| h.0 as usize)
                .filter(|&h| h != 0)
        };

        let has_focus = unsafe {
            self.raw
                .CachedHasKeyboardFocus()
                .or_else(|_| self.raw.CurrentHasKeyboardFocus())
                .unwrap_or_default()
                .as_bool()
        };

        let mut states = State::empty();
        if has_focus {
            states |= State::FOCUSED;
        }

        let is_offscreen = unsafe {
            self.raw
                .GetCachedPropertyValue(UIA_IsOffscreenPropertyId)
                .or_else(|_| self.raw.GetCurrentPropertyValue(UIA_IsOffscreenPropertyId))
                .ok()
                .and_then(|v| bool::try_from(&v).ok())
                .unwrap_or(false)
        };
        if is_offscreen {
            states |= State::OFFSCREEN;
        }

        // Check TogglePattern toggle state (checked, checkable, halfchecked)
        let toggle_state = unsafe {
            self.raw
                .GetCachedPropertyValue(UIA_ToggleToggleStatePropertyId)
                .or_else(|_| self.raw.GetCurrentPropertyValue(UIA_ToggleToggleStatePropertyId))
                .ok()
                .and_then(|v| i32::try_from(&v).ok())
        };
        match toggle_state {
            Some(0) => states |= State::CHECKABLE,
            Some(1) => states |= State::CHECKED,
            Some(2) => states |= State::HALFCHECKED,
            _ => {}
        }

        // Check ExpandCollapsePattern state (expanded, collapsed)
        let expand_state = unsafe {
            self.raw
                .GetCachedPropertyValue(UIA_ExpandCollapseExpandCollapseStatePropertyId)
                .or_else(|_| self.raw.GetCurrentPropertyValue(UIA_ExpandCollapseExpandCollapseStatePropertyId))
                .ok()
                .and_then(|v| i32::try_from(&v).ok())
        };
        match expand_state {
            Some(0) => states |= State::COLLAPSED,
            Some(1) => states |= State::EXPANDED,
            _ => {}
        }

        // Check SelectionItem pattern (selected)
        let is_selected = unsafe {
            self.raw
                .GetCachedPropertyValue(UIA_SelectionItemIsSelectedPropertyId)
                .or_else(|_| self.raw.GetCurrentPropertyValue(UIA_SelectionItemIsSelectedPropertyId))
                .ok()
                .and_then(|v| bool::try_from(&v).ok())
                .unwrap_or(false)
        };
        if is_selected {
            states |= State::SELECTED;
        }

        // Check IsEnabled (unavailable / disabled)
        let is_enabled = unsafe {
            self.raw
                .GetCachedPropertyValue(UIA_IsEnabledPropertyId)
                .or_else(|_| self.raw.GetCurrentPropertyValue(UIA_IsEnabledPropertyId))
                .ok()
                .and_then(|v| bool::try_from(&v).ok())
                .unwrap_or(true)
        };
        if !is_enabled {
            states |= State::UNAVAILABLE;
        }

        // Check IsReadOnly
        let is_readonly = unsafe {
            self.raw
                .GetCachedPropertyValue(UIA_ValueIsReadOnlyPropertyId)
                .or_else(|_| self.raw.GetCurrentPropertyValue(UIA_ValueIsReadOnlyPropertyId))
                .ok()
                .and_then(|v| bool::try_from(&v).ok())
                .unwrap_or(false)
        };
        if is_readonly {
            states |= State::READONLY;
        }

        let position_in_set = unsafe {
            self.raw
                .GetCachedPropertyValue(UIA_PositionInSetPropertyId)
                .or_else(|_| self.raw.GetCurrentPropertyValue(UIA_PositionInSetPropertyId))
                .ok()
                .and_then(|v| i32::try_from(&v).ok())
                .filter(|&val| val > 0)
        };

        let size_of_set = unsafe {
            self.raw
                .GetCachedPropertyValue(UIA_SizeOfSetPropertyId)
                .or_else(|_| self.raw.GetCurrentPropertyValue(UIA_SizeOfSetPropertyId))
                .ok()
                .and_then(|v| i32::try_from(&v).ok())
                .filter(|&val| val > 0)
        };

        let mut level = unsafe {
            self.raw
                .GetCachedPropertyValue(UIA_LevelPropertyId)
                .or_else(|_| self.raw.GetCurrentPropertyValue(UIA_LevelPropertyId))
                .ok()
                .and_then(|v| i32::try_from(&v).ok())
                .filter(|&val| val > 0)
        };

        // If level is not set, try parsing level from aria-properties
        if level.is_none() {
            let aria_props = unsafe {
                self.raw
                    .GetCachedPropertyValue(UIA_AriaPropertiesPropertyId)
                    .or_else(|_| self.raw.GetCurrentPropertyValue(UIA_AriaPropertiesPropertyId))
                    .ok()
                    .and_then(|v| variant_to_string(&v))
            };
            if let Some(props) = aria_props {
                for part in props.split(';') {
                    let mut kv = part.split('=');
                    if let (Some(k), Some(v)) = (kv.next(), kv.next()) {
                        if k.trim() == "level" {
                            if let Ok(l) = v.trim().parse::<i32>() {
                                level = Some(l);
                                break;
                            }
                        }
                    }
                }
            }
        }

        if level.is_some() && (role == Role::Unknown || role == Role::Header || role == Role::Grouping) {
            role = Role::Heading;
        }

        let bounds = unsafe {
            self.raw
                .CachedBoundingRectangle()
                .or_else(|_| self.raw.CurrentBoundingRectangle())
                .ok()
                .map(|r| Rect {
                    left: r.left as f64,
                    top: r.top as f64,
                    width: (r.right - r.left) as f64,
                    height: (r.bottom - r.top) as f64,
                })
        };

        let value = unsafe {
            self.raw
                .GetCachedPropertyValue(UIA_ValueValuePropertyId)
                .or_else(|_| self.raw.GetCurrentPropertyValue(UIA_ValueValuePropertyId))
                .ok()
                .and_then(|v| variant_to_string(&v))
                .filter(|s| !s.trim().is_empty())
                .or_else(|| {
                    self.raw
                        .GetCachedPropertyValue(UIA_RangeValueValuePropertyId)
                        .or_else(|_| self.raw.GetCurrentPropertyValue(UIA_RangeValueValuePropertyId))
                        .ok()
                        .and_then(|v| f64::try_from(&v).ok())
                        .map(|val| format!("{}", val))
                })
        };

        let description = unsafe {
            self.raw
                .GetCachedPropertyValue(UIA_FullDescriptionPropertyId)
                .or_else(|_| self.raw.GetCurrentPropertyValue(UIA_FullDescriptionPropertyId))
                .or_else(|_| self.raw.GetCachedPropertyValue(UIA_HelpTextPropertyId))
                .or_else(|_| self.raw.GetCurrentPropertyValue(UIA_HelpTextPropertyId))
                .ok()
                .and_then(|v| variant_to_string(&v))
                .filter(|s| !s.trim().is_empty())
        };

        let keyboard_shortcut = unsafe {
            self.raw
                .GetCachedPropertyValue(UIA_AccessKeyPropertyId)
                .or_else(|_| self.raw.GetCurrentPropertyValue(UIA_AccessKeyPropertyId))
                .or_else(|_| self.raw.GetCachedPropertyValue(UIA_AcceleratorKeyPropertyId))
                .or_else(|_| self.raw.GetCurrentPropertyValue(UIA_AcceleratorKeyPropertyId))
                .ok()
                .and_then(|v| variant_to_string(&v))
                .filter(|s| !s.trim().is_empty())
        };

        let id = unsafe {
            if let Ok(runtime_id) = self.raw.GetRuntimeId() {
                if let Some(hash) = extract_runtime_id_hash(runtime_id) {
                    NodeId(hash)
                } else if let Some(h) = hwnd {
                    NodeId(h as u64)
                } else if let Some(ref aid) = automation_id {
                    let mut h: u64 = 0xcbf29ce484222325;
                    for byte in aid.bytes() {
                        h ^= byte as u64;
                        h = h.wrapping_mul(0x100000001b3);
                    }
                    NodeId(h)
                } else {
                    NodeId(self.raw.as_raw() as usize as u64)
                }
            } else if let Some(h) = hwnd {
                NodeId(h as u64)
            } else if let Some(ref aid) = automation_id {
                let mut h: u64 = 0xcbf29ce484222325;
                for byte in aid.bytes() {
                    h ^= byte as u64;
                    h = h.wrapping_mul(0x100000001b3);
                }
                NodeId(h)
            } else {
                NodeId(self.raw.as_raw() as usize as u64)
            }
        };

        let framework_id = unsafe {
            self.raw
                .GetCachedPropertyValue(UIA_FrameworkIdPropertyId)
                .or_else(|_| self.raw.GetCurrentPropertyValue(UIA_FrameworkIdPropertyId))
                .ok()
                .and_then(|v| variant_to_string(&v))
        };

        let is_web_content = aria_role.is_some()
            || framework_id.as_deref() == Some("Chrome")
            || framework_id.as_deref() == Some("Gecko")
            || class_name
                .as_deref()
                .map(|c| {
                    c == "Chrome_RenderWidgetHostHWND"
                        || c == "MozillaContentWindowClass"
                        || c == "Intermediate D3D Window"
                        || c.contains("WebView")
                        || c.contains("RenderWidget")
                        || c == "Internet Explorer_Server"
                })
                .unwrap_or(false)
            || hwnd
                .map(|h| {
                    let win = windows::Win32::Foundation::HWND(h as _);
                    let mut class_buf = [0u16; 256];
                    let len = unsafe {
                        windows::Win32::UI::WindowsAndMessaging::GetClassNameW(win, &mut class_buf)
                    };
                    if len > 0 {
                        let win_cls = String::from_utf16_lossy(&class_buf[..len as usize]);
                        win_cls == "Chrome_RenderWidgetHostHWND"
                            || win_cls == "MozillaContentWindowClass"
                            || win_cls == "Intermediate D3D Window"
                            || win_cls.contains("WebView")
                            || win_cls.contains("RenderWidget")
                            || win_cls == "Internet Explorer_Server"
                    } else {
                        false
                    }
                })
                .unwrap_or(false)
            || process_id
                .and_then(crate::apps::get_process_name_by_pid)
                .map(|proc_name| {
                    let proc_lower = proc_name.to_lowercase();
                    let is_webview = proc_lower.contains("webview") || proc_lower == "msedgewebview2.exe";
                    let is_browser = crate::apps::ChromiumFilter::is_chromium_process(&proc_name)
                        || crate::apps::FirefoxFilter::is_gecko_process(&proc_name);

                    if is_webview {
                        true
                    } else if is_browser {
                        matches!(
                            role,
                            Role::Document
                                | Role::Frame
                                | Role::Heading
                                | Role::Link
                                | Role::Paragraph
                                | Role::Button
                                | Role::CheckBox
                                | Role::RadioButton
                                | Role::EditableText
                                | Role::ComboBox
                                | Role::List
                                | Role::ListItem
                                | Role::Table
                                | Role::StaticText
                                | Role::Graphic
                        )
                    } else {
                        false
                    }
                })
                .unwrap_or(false);

        AccessibleNode {
            id,
            role,
            states,
            name,
            value,
            description,
            keyboard_shortcut,
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
            is_web_content,
            ..Default::default()
        }
    }
}
