//! Safe Wrappers for Core UI Automation Patterns.
//! Aligned with WINDOWS.md Section 3.6.

use crate::error::Result;
use windows::core::BSTR;
use windows::core::Interface;
use windows::Win32::UI::Accessibility::*;

pub struct Patterns<'a> {
    element: &'a IUIAutomationElement,
}

impl<'a> Patterns<'a> {
    pub fn new(element: &'a IUIAutomationElement) -> Self {
        Self { element }
    }

    /// Invokes the primary action on a button, split-button, or link.
    pub fn invoke(&self) -> Result<()> {
        unsafe {
            let pattern = self.element.GetCurrentPattern(UIA_InvokePatternId)?;
            let invoke: IUIAutomationInvokePattern = pattern.cast()?;
            invoke.Invoke()?;
            Ok(())
        }
    }

    /// Toggles a checkbox or toggle button (On / Off).
    pub fn toggle(&self) -> Result<()> {
        unsafe {
            let pattern = self.element.GetCurrentPattern(UIA_TogglePatternId)?;
            let toggle: IUIAutomationTogglePattern = pattern.cast()?;
            toggle.Toggle()?;
            Ok(())
        }
    }

    /// Expands or collapses a tree item, combo box, or expander control.
    pub fn set_expanded(&self, expand: bool) -> Result<()> {
        unsafe {
            let pattern = self.element.GetCurrentPattern(UIA_ExpandCollapsePatternId)?;
            let ec: IUIAutomationExpandCollapsePattern = pattern.cast()?;
            if expand {
                ec.Expand()?;
            } else {
                ec.Collapse()?;
            }
            Ok(())
        }
    }

    /// Selects an item in a list or tab strip.
    pub fn select_item(&self) -> Result<()> {
        unsafe {
            let pattern = self.element.GetCurrentPattern(UIA_SelectionItemPatternId)?;
            let item: IUIAutomationSelectionItemPattern = pattern.cast()?;
            item.Select()?;
            Ok(())
        }
    }

    /// Reads plain text from an edit control.
    pub fn get_value(&self) -> Result<String> {
        unsafe {
            let pattern = self.element.GetCurrentPattern(UIA_ValuePatternId)?;
            let val: IUIAutomationValuePattern = pattern.cast()?;
            let bstr: BSTR = val.CurrentValue()?;
            Ok(bstr.to_string())
        }
    }

    /// Sets text into an edit control.
    pub fn set_value(&self, text: &str) -> Result<()> {
        unsafe {
            let pattern = self.element.GetCurrentPattern(UIA_ValuePatternId)?;
            let val: IUIAutomationValuePattern = pattern.cast()?;
            let bstr = BSTR::from(text);
            val.SetValue(&bstr)?;
            Ok(())
        }
    }
}
