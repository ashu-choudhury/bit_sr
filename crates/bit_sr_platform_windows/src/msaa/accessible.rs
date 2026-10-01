//! Safe Wrapper for MSAA IAccessible Interface and Child IDs.

use crate::msaa::roles_states::{msaa_role_to_role, msaa_state_to_state};
use bit_sr_core::node::{AccessibleNode, NodeId};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::IAccessible;

unsafe fn variant_to_string(v: &VARIANT) -> Option<String> {
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

pub struct MsaaElement {
    acc: IAccessible,
    var_child: VARIANT,
    child_id: i32,
    hwnd: usize,
}

impl MsaaElement {
    pub fn new(acc: IAccessible, var_child: VARIANT, child_id: i32, hwnd: usize) -> Self {
        Self {
            acc,
            var_child,
            child_id,
            hwnd,
        }
    }

    /// Converts this MSAA accessible into a unified AccessibleNode.
    pub fn to_accessible_node(&self) -> AccessibleNode {
        let name = unsafe {
            self.acc.get_accName(&self.var_child).ok().and_then(|bstr| {
                let s = bstr.to_string();
                let trimmed = s.trim();
                // Filter out raw child ID numbers reported as names (e.g. "4", "-2147483647")
                if trimmed.is_empty() || trimmed.parse::<i64>().is_ok() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            })
        };

        let role = unsafe {
            if let Ok(var_role) = self.acc.get_accRole(&self.var_child) {
                if let Ok(role_id) = u32::try_from(&var_role) {
                    msaa_role_to_role(role_id)
                } else if let Some(role_str) = variant_to_string(&var_role) {
                    match role_str.trim().to_ascii_lowercase().as_str() {
                        "heading" => bit_sr_core::roles::Role::Heading,
                        "button" => bit_sr_core::roles::Role::Button,
                        "link" => bit_sr_core::roles::Role::Link,
                        "check box" | "checkbox" => bit_sr_core::roles::Role::CheckBox,
                        "radio button" | "radio" => bit_sr_core::roles::Role::RadioButton,
                        _ => bit_sr_core::roles::Role::Unknown,
                    }
                } else {
                    bit_sr_core::roles::Role::Unknown
                }
            } else {
                bit_sr_core::roles::Role::Unknown
            }
        };

        let states = unsafe {
            if let Ok(var_state) = self.acc.get_accState(&self.var_child) {
                let state_id = u32::try_from(&var_state).unwrap_or(0);
                msaa_state_to_state(state_id)
            } else {
                bit_sr_core::states::State::empty()
            }
        };

        let value = unsafe {
            self.acc.get_accValue(&self.var_child).ok().and_then(|bstr| {
                let s = bstr.to_string();
                if s.trim().is_empty() { None } else { Some(s) }
            })
        };

        let description = unsafe {
            self.acc.get_accDescription(&self.var_child).ok().and_then(|bstr| {
                let s = bstr.to_string();
                if s.trim().is_empty() { None } else { Some(s) }
            })
        };

        let id = NodeId((self.hwnd as u64) << 32 | (self.child_id as u32 as u64));

        AccessibleNode {
            id,
            role,
            states,
            name,
            value,
            description,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use windows::Win32::System::Variant::VARIANT;

    #[test]
    fn test_var_role_try_from() {
        let var_i32 = VARIANT::from(43i32);
        let res_u32 = u32::try_from(&var_i32);
        let res_i32 = i32::try_from(&var_i32);
        assert_eq!(res_u32.ok(), Some(43));
        assert_eq!(res_i32.ok(), Some(43));
    }
}

