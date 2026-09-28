//! Safe Wrapper for MSAA IAccessible Interface and Child IDs.

use crate::msaa::roles_states::{msaa_role_to_role, msaa_state_to_state};
use bit_sr_core::node::{AccessibleNode, NodeId};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::IAccessible;

pub struct MsaaElement {
    acc: IAccessible,
    child_id: i32,
    hwnd: usize,
}

impl MsaaElement {
    pub fn new(acc: IAccessible, child_id: i32, hwnd: usize) -> Self {
        Self { acc, child_id, hwnd }
    }

    /// Converts this MSAA accessible into a unified AccessibleNode.
    pub fn to_accessible_node(&self) -> AccessibleNode {
        let var_child = VARIANT::from(self.child_id);

        let name = unsafe {
            self.acc.get_accName(&var_child).ok().and_then(|bstr| {
                let s = bstr.to_string();
                if s.trim().is_empty() { None } else { Some(s) }
            })
        };

        let role = unsafe {
            if let Ok(var_role) = self.acc.get_accRole(&var_child) {
                let role_id = u32::try_from(&var_role).unwrap_or(0);
                msaa_role_to_role(role_id)
            } else {
                bit_sr_core::roles::Role::Unknown
            }
        };

        let states = unsafe {
            if let Ok(var_state) = self.acc.get_accState(&var_child) {
                let state_id = u32::try_from(&var_state).unwrap_or(0);
                msaa_state_to_state(state_id)
            } else {
                bit_sr_core::states::State::empty()
            }
        };

        let value = unsafe {
            self.acc.get_accValue(&var_child).ok().and_then(|bstr| {
                let s = bstr.to_string();
                if s.trim().is_empty() { None } else { Some(s) }
            })
        };

        let description = unsafe {
            self.acc.get_accDescription(&var_child).ok().and_then(|bstr| {
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
