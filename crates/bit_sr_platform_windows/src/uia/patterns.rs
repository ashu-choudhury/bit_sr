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

    /// Reads text at the current caret / selection position using IUIAutomationTextPattern2 / IUIAutomationTextPattern.
    /// Supports TextUnit_Character, TextUnit_Word, and TextUnit_Line.
    pub fn get_text_at_caret(&self, unit: TextUnit) -> Result<Option<String>> {
        unsafe {
            // 1. Try TextPattern2 first (Windows 8.1+, modern XAML / Windows 11 Notepad / Chromium)
            if let Ok(pattern2) = self.element.GetCurrentPattern(UIA_TextPattern2Id) {
                if let Ok(text_pat2) = pattern2.cast::<IUIAutomationTextPattern2>() {
                    let mut is_active = windows::core::BOOL(0);
                    if let Ok(range) = text_pat2.GetCaretRange(&mut is_active) {
                        let caret_pos = range.Clone()?;
                        let _ = range.ExpandToEnclosingUnit(unit);
                        if unit == TextUnit_Character {
                            if let Ok(cmp) = range.CompareEndpoints(
                                TextPatternRangeEndpoint_Start,
                                &caret_pos,
                                TextPatternRangeEndpoint_Start,
                            ) {
                                if cmp < 0 {
                                    // Caret is past the character (end of line / text) -> Blank
                                    return Ok(Some(String::new()));
                                }
                            }
                        }
                        if let Ok(bstr) = range.GetText(-1) {
                            return Ok(Some(bstr.to_string()));
                        }
                    }
                }
            }

            // 2. Fallback to standard TextPattern GetSelection()
            let pattern = self.element.GetCurrentPattern(UIA_TextPatternId)?;
            let text_pat: IUIAutomationTextPattern = pattern.cast()?;
            let selection = text_pat.GetSelection()?;
            if selection.Length()? > 0 {
                let range = selection.GetElement(0)?;
                let caret_pos = range.Clone()?;
                range.ExpandToEnclosingUnit(unit)?;
                if unit == TextUnit_Character {
                    if let Ok(cmp) = range.CompareEndpoints(
                        TextPatternRangeEndpoint_Start,
                        &caret_pos,
                        TextPatternRangeEndpoint_Start,
                    ) {
                        if cmp < 0 {
                            return Ok(Some(String::new()));
                        }
                    }
                }
                let bstr = range.GetText(-1)?;
                let text = bstr.to_string();
                return Ok(Some(text));
            }
            Ok(None)
        }
    }

    /// Reads currently selected text using IUIAutomationTextPattern.
    pub fn get_selected_text(&self) -> Result<Option<String>> {
        unsafe {
            let pattern = self.element.GetCurrentPattern(UIA_TextPatternId)?;
            let text_pat: IUIAutomationTextPattern = pattern.cast()?;
            let selection = text_pat.GetSelection()?;
            if selection.Length()? > 0 {
                let range = selection.GetElement(0)?;
                let bstr = range.GetText(-1)?;
                let text = bstr.to_string();
                if !text.is_empty() {
                    return Ok(Some(text));
                }
            }
            Ok(None)
        }
    }

    /// Reads full document or console visible text using IUIAutomationTextPattern.
    /// In modern Windows consoles (conhost) and Windows Terminal, GetVisibleRanges()
    /// provides the active viewport, and DocumentRange() provides the entire buffer.
    pub fn get_document_text(&self) -> Result<Option<String>> {
        unsafe {
            if let Ok(pattern) = self.element.GetCurrentPattern(UIA_TextPatternId) {
                if let Ok(text_pat) = pattern.cast::<IUIAutomationTextPattern>() {
                    // 1. Try GetVisibleRanges first (ideal for terminal windows where buffer has thousands of blanks)
                    if let Ok(visible_ranges) = text_pat.GetVisibleRanges() {
                        if visible_ranges.Length()? > 0 {
                            let mut full = String::new();
                            for i in 0..visible_ranges.Length()? {
                                if let Ok(range) = visible_ranges.GetElement(i) {
                                    if let Ok(bstr) = range.GetText(-1) {
                                        full.push_str(&bstr.to_string());
                                    }
                                }
                            }
                            if !full.trim().is_empty() {
                                return Ok(Some(full));
                            }
                        }
                    }
                    // 2. Fall back to DocumentRange
                    if let Ok(doc_range) = text_pat.DocumentRange() {
                        if let Ok(bstr) = doc_range.GetText(-1) {
                            let text = bstr.to_string();
                            if !text.trim().is_empty() {
                                return Ok(Some(text));
                            }
                        }
                    }
                }
            }

            // 3. Fallback to ValuePattern for simple edit controls
            if let Ok(pattern) = self.element.GetCurrentPattern(UIA_ValuePatternId) {
                if let Ok(val_pat) = pattern.cast::<IUIAutomationValuePattern>() {
                    if let Ok(bstr) = val_pat.CurrentValue() {
                        let text = bstr.to_string();
                        if !text.is_empty() {
                            return Ok(Some(text));
                        }
                    }
                }
            }

            Ok(None)
        }
    }

    /// Reads caret insertion point character offset within the document using IUIAutomationTextPattern.
    pub fn get_caret_offset(&self) -> Result<Option<usize>> {
        unsafe {
            if let Ok(pattern) = self.element.GetCurrentPattern(UIA_TextPatternId) {
                if let Ok(text_pat) = pattern.cast::<IUIAutomationTextPattern>() {
                    let selection = text_pat.GetSelection()?;
                    if selection.Length()? > 0 {
                        let caret_range = selection.GetElement(0)?;
                        let doc_range = text_pat.DocumentRange()?;
                        let target = doc_range.Clone()?;
                        let _ = target.MoveEndpointByRange(
                            TextPatternRangeEndpoint_End,
                            &caret_range,
                            TextPatternRangeEndpoint_Start,
                        );
                        if let Ok(bstr) = target.GetText(-1) {
                            return Ok(Some(bstr.to_string().chars().count()));
                        }
                    }
                }
            }
            Ok(None)
        }
    }
}
