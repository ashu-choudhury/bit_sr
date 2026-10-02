//! Windows Native Caret Text Provider.
//! Queries text at the caret via UIA IUIAutomationTextPattern / TextPattern2 with Win32 Edit fallback.

use crate::common_controls::EditControlReader;
use crate::uia::patterns::Patterns;
use crate::uia::UiaClient;
use bit_sr_core::text::{TextProvider, TextUnit};
use std::sync::Arc;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Accessibility::*;
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetGUIThreadInfo, GUITHREADINFO,
};

pub struct WindowsTextProvider {
    uia: Arc<UiaClient>,
}

impl WindowsTextProvider {
    pub fn new(uia: Arc<UiaClient>) -> Self {
        Self { uia }
    }

    /// Obtains the handle of the actual focused child window across any process in Windows.
    fn get_focused_hwnd() -> HWND {
        unsafe {
            let mut gui_info = GUITHREADINFO {
                cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
                ..Default::default()
            };
            if GetGUIThreadInfo(0, &mut gui_info).is_ok() && !gui_info.hwndFocus.0.is_null() {
                gui_info.hwndFocus
            } else {
                GetForegroundWindow()
            }
        }
    }
    /// Obtains text at caret from the element or its nearest ancestors (e.g. for PDF documents / frames).
    fn get_text_from_element_or_ancestors(
        &self,
        elem: &windows::Win32::UI::Accessibility::IUIAutomationElement,
        unit: windows::Win32::UI::Accessibility::TextUnit,
    ) -> Option<String> {
        let patterns = Patterns::new(elem);
        if let Ok(Some(text)) = patterns.get_text_at_caret(unit) {
            return Some(text);
        }
        if let Ok(nav) = self.uia.control_view_navigator() {
            let mut current = elem.clone();
            for _ in 0..3 {
                if let Some(parent) = nav.get_parent(&current) {
                    let pat = Patterns::new(&parent);
                    if let Ok(Some(text)) = pat.get_text_at_caret(unit) {
                        return Some(text);
                    }
                    current = parent;
                } else {
                    break;
                }
            }
        }
        None
    }

    /// Obtains selected text from the element or its nearest ancestors.
    fn get_selection_from_element_or_ancestors(
        &self,
        elem: &windows::Win32::UI::Accessibility::IUIAutomationElement,
    ) -> Option<String> {
        let patterns = Patterns::new(elem);
        if let Ok(Some(text)) = patterns.get_selected_text() {
            return Some(text);
        }
        if let Ok(nav) = self.uia.control_view_navigator() {
            let mut current = elem.clone();
            for _ in 0..3 {
                if let Some(parent) = nav.get_parent(&current) {
                    let pat = Patterns::new(&parent);
                    if let Ok(Some(text)) = pat.get_selected_text() {
                        return Some(text);
                    }
                    current = parent;
                } else {
                    break;
                }
            }
        }
        None
    }

    /// Obtains full document or visible terminal text from the element or its nearest ancestors.
    fn get_document_from_element_or_ancestors(
        &self,
        elem: &windows::Win32::UI::Accessibility::IUIAutomationElement,
    ) -> Option<String> {
        let patterns = Patterns::new(elem);
        if let Ok(Some(text)) = patterns.get_document_text() {
            return Some(text);
        }
        if let Ok(nav) = self.uia.control_view_navigator() {
            let mut current = elem.clone();
            for _ in 0..3 {
                if let Some(parent) = nav.get_parent(&current) {
                    let pat = Patterns::new(&parent);
                    if let Ok(Some(text)) = pat.get_document_text() {
                        return Some(text);
                    }
                    current = parent;
                } else {
                    break;
                }
            }
        }
        None
    }
}

impl TextProvider for WindowsTextProvider {
    fn get_text_at_caret(&self, unit: TextUnit) -> Option<String> {
        let uia_unit = match unit {
            TextUnit::Character => TextUnit_Character,
            TextUnit::Word => TextUnit_Word,
            TextUnit::Line => TextUnit_Line,
            TextUnit::Paragraph => TextUnit_Paragraph,
            TextUnit::Document => TextUnit_Document,
        };

        // 1. Try UIA TextPattern on the focused element and its ancestors
        if let Ok(elem) = self.uia.get_focused_element() {
            if let Some(text) = self.get_text_from_element_or_ancestors(elem.raw(), uia_unit) {
                return Some(text);
            }
        }

        let focus_hwnd = Self::get_focused_hwnd();

        // 2. Try UIA TextPattern on the element obtained directly from the focused HWND
        if !focus_hwnd.0.is_null() {
            if let Ok(elem) = self.uia.element_from_handle(focus_hwnd) {
                if let Some(text) = self.get_text_from_element_or_ancestors(elem.raw(), uia_unit) {
                    return Some(text);
                }
            }
        }

        // 3. Fallback to classic Win32 Edit / RichEdit control
        if !focus_hwnd.0.is_null() {
            if let Some(text) = EditControlReader::get_text_at_caret(focus_hwnd, unit) {
                return Some(text);
            }
        }

        None
    }

    fn get_selected_text(&self) -> Option<String> {
        // 1. Try UIA TextPattern on focused element and its ancestors
        if let Ok(elem) = self.uia.get_focused_element() {
            if let Some(text) = self.get_selection_from_element_or_ancestors(elem.raw()) {
                return Some(text);
            }
        }

        let focus_hwnd = Self::get_focused_hwnd();

        // 2. Try UIA TextPattern on element from focused HWND
        if !focus_hwnd.0.is_null() {
            if let Ok(elem) = self.uia.element_from_handle(focus_hwnd) {
                if let Some(text) = self.get_selection_from_element_or_ancestors(elem.raw()) {
                    return Some(text);
                }
            }
        }

        // 3. Fallback to classic Win32 Edit control
        if !focus_hwnd.0.is_null() {
            if let Some(text) = EditControlReader::get_selected_text(focus_hwnd) {
                return Some(text);
            }
        }

        None
    }

    fn get_document_text(&self) -> Option<String> {
        // 1. Try UIA TextPattern on focused element and ancestors (e.g. terminals, documents)
        if let Ok(elem) = self.uia.get_focused_element() {
            if let Some(text) = self.get_document_from_element_or_ancestors(elem.raw()) {
                return Some(text);
            }
        }

        let focus_hwnd = Self::get_focused_hwnd();

        // 2. Try UIA TextPattern on element from focused HWND
        if !focus_hwnd.0.is_null() {
            if let Ok(elem) = self.uia.element_from_handle(focus_hwnd) {
                if let Some(text) = self.get_document_from_element_or_ancestors(elem.raw()) {
                    return Some(text);
                }
            }
        }

        // 3. Fallback to classic Win32 Edit control full text
        if !focus_hwnd.0.is_null() {
            if let Some(text) = EditControlReader::get_window_text(focus_hwnd) {
                if !text.is_empty() {
                    return Some(text);
                }
            }
        }

        None
    }

    fn get_caret_offset(&self) -> Option<usize> {
        // 1. Try UIA on focused element
        if let Ok(elem) = self.uia.get_focused_element() {
            let pat = Patterns::new(elem.raw());
            if let Ok(Some(offset)) = pat.get_caret_offset() {
                return Some(offset);
            }
        }

        let focus_hwnd = Self::get_focused_hwnd();

        // 2. Try Win32 Edit control
        if !focus_hwnd.0.is_null() {
            if let Some((start, _)) = EditControlReader::get_selection(focus_hwnd) {
                return Some(start);
            }
        }

        None
    }
}
