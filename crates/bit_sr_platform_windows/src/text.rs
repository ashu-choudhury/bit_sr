//! Windows Native Caret Text Provider.
//! Queries text at the caret via UIA IUIAutomationTextPattern with Win32 Edit fallback.

use crate::common_controls::EditControlReader;
use crate::uia::patterns::Patterns;
use crate::uia::UiaClient;
use bit_sr_core::text::{TextProvider, TextUnit};
use std::sync::Arc;
use windows::Win32::UI::Accessibility::*;
use windows::Win32::UI::Input::KeyboardAndMouse::GetFocus;
use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

pub struct WindowsTextProvider {
    uia: Arc<UiaClient>,
}

impl WindowsTextProvider {
    pub fn new(uia: Arc<UiaClient>) -> Self {
        Self { uia }
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

        // 1. Try UIA TextPattern on the focused element
        if let Ok(elem) = self.uia.get_focused_element() {
            let patterns = Patterns::new(elem.raw());
            if let Ok(Some(text)) = patterns.get_text_at_caret(uia_unit) {
                return Some(text);
            }
        }

        // 2. Fallback to classic Win32 Edit / RichEdit control
        unsafe {
            let focus_hwnd = GetFocus();
            let target_hwnd = if !focus_hwnd.0.is_null() {
                focus_hwnd
            } else {
                GetForegroundWindow()
            };

            if !target_hwnd.0.is_null() {
                if let Some(text) = EditControlReader::get_text_at_caret(target_hwnd, unit) {
                    return Some(text);
                }
            }
        }

        None
    }
}
