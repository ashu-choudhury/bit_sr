//! Edit and RichEdit Common Controls Interrogation.
//! Implements Section 5.1 of WINDOWS.md.

use crate::watchdog::safe_send_message_timeout;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::Controls::{
    EM_GETSEL, EM_LINEFROMCHAR, EM_LINEINDEX, EM_LINELENGTH,
};

pub struct EditControlReader;

impl EditControlReader {
    /// Obtains (selection_start, selection_end) offsets from an edit control.
    pub fn get_selection(hwnd: HWND) -> Option<(usize, usize)> {
        let mut start: u32 = 0;
        let mut end: u32 = 0;

        let res = safe_send_message_timeout(
            hwnd,
            EM_GETSEL,
            WPARAM(&mut start as *mut _ as usize),
            LPARAM(&mut end as *mut _ as isize),
            500,
        );

        if res.is_some() {
            Some((start as usize, end as usize))
        } else {
            None
        }
    }

    /// Maps a character offset to its 0-based line index.
    pub fn get_line_from_char(hwnd: HWND, char_index: usize) -> Option<usize> {
        safe_send_message_timeout(hwnd, EM_LINEFROMCHAR, WPARAM(char_index), LPARAM(0), 500)
    }

    /// Obtains character offset where the given line begins.
    pub fn get_line_index(hwnd: HWND, line_index: usize) -> Option<usize> {
        safe_send_message_timeout(hwnd, EM_LINEINDEX, WPARAM(line_index), LPARAM(0), 500)
    }

    /// Returns character length of the specified line.
    pub fn get_line_length(hwnd: HWND, char_index_in_line: usize) -> Option<usize> {
        safe_send_message_timeout(hwnd, EM_LINELENGTH, WPARAM(char_index_in_line), LPARAM(0), 500)
    }

    /// Obtains the text of the edit control, working across process boundaries via WM_GETTEXTLENGTH and WM_GETTEXT.
    pub fn get_window_text(hwnd: HWND) -> Option<String> {
        const WM_GETTEXT: u32 = 0x000D;
        const WM_GETTEXTLENGTH: u32 = 0x000E;

        if let Some(len) = safe_send_message_timeout(hwnd, WM_GETTEXTLENGTH, WPARAM(0), LPARAM(0), 500) {
            if len == 0 {
                return Some(String::new());
            }
            let mut buf = vec![0u16; len + 2];
            if let Some(copied) = safe_send_message_timeout(
                hwnd,
                WM_GETTEXT,
                WPARAM(buf.len()),
                LPARAM(buf.as_mut_ptr() as isize),
                500,
            ) {
                if copied > 0 {
                    return Some(String::from_utf16_lossy(&buf[..copied]));
                }
            }
        }

        // Fallback to GetWindowTextW
        let mut buf = [0u16; 4096];
        let len = unsafe { windows::Win32::UI::WindowsAndMessaging::GetWindowTextW(hwnd, &mut buf) };
        if len > 0 {
            Some(String::from_utf16_lossy(&buf[..len as usize]))
        } else {
            None
        }
    }

    /// Obtains currently selected text in a classic Win32 edit control.
    pub fn get_selected_text(hwnd: HWND) -> Option<String> {
        let (start, end) = Self::get_selection(hwnd)?;
        if start == end {
            return None;
        }
        let full_text = Self::get_window_text(hwnd)?;
        let chars: Vec<char> = full_text.chars().collect();
        let s = start.min(chars.len());
        let e = end.min(chars.len());
        if s < e {
            Some(chars[s..e].iter().collect())
        } else {
            None
        }
    }

    /// Extracts text at the caret for the given unit (Character, Word, or Line) in a classic Win32 edit control.
    pub fn get_text_at_caret(hwnd: HWND, unit: bit_sr_core::TextUnit) -> Option<String> {
        let (start, _end) = Self::get_selection(hwnd)?;
        let full_text = Self::get_window_text(hwnd)?;
        let chars: Vec<char> = full_text.chars().collect();

        match unit {
            bit_sr_core::TextUnit::Character => {
                if start < chars.len() {
                    Some(chars[start].to_string())
                } else {
                    Some(String::new())
                }
            }
            bit_sr_core::TextUnit::Line => {
                Some(Self::extract_line_at_offset(&full_text, start))
            }
            bit_sr_core::TextUnit::Word => {
                Some(Self::extract_word_at_offset(&full_text, start))
            }
            _ => Some(full_text),
        }
    }

    /// Extracts the line of text spanning the given character offset.
    pub fn extract_line_at_offset(text: &str, offset: usize) -> String {
        let mut current_offset = 0;
        for line in text.split('\n') {
            let line_len = line.len() + 1; // +1 for newline character
            if offset >= current_offset && offset < current_offset + line_len {
                return line.trim_end_matches('\r').to_string();
            }
            current_offset += line_len;
        }
        text.lines().last().unwrap_or("").trim_end_matches('\r').to_string()
    }

    /// Extracts the word spanning the given character offset.
    pub fn extract_word_at_offset(text: &str, offset: usize) -> String {
        let chars: Vec<char> = text.chars().collect();
        if chars.is_empty() || offset >= chars.len() {
            return String::new();
        }

        let mut start = offset;
        while start > 0 && !chars[start - 1].is_whitespace() {
            start -= 1;
        }

        let mut end = offset;
        while end < chars.len() && !chars[end].is_whitespace() {
            end += 1;
        }

        chars[start..end].iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_line_at_offset() {
        let text = "First line\nSecond line here\nThird line";
        assert_eq!(EditControlReader::extract_line_at_offset(text, 0), "First line");
        assert_eq!(EditControlReader::extract_line_at_offset(text, 5), "First line");
        assert_eq!(EditControlReader::extract_line_at_offset(text, 11), "Second line here");
        assert_eq!(EditControlReader::extract_line_at_offset(text, 20), "Second line here");
        assert_eq!(EditControlReader::extract_line_at_offset(text, 28), "Third line");
    }

    #[test]
    fn test_extract_word_at_offset() {
        let text = "Hello world from bit_sr";
        assert_eq!(EditControlReader::extract_word_at_offset(text, 0), "Hello");
        assert_eq!(EditControlReader::extract_word_at_offset(text, 3), "Hello");
        assert_eq!(EditControlReader::extract_word_at_offset(text, 6), "world");
        assert_eq!(EditControlReader::extract_word_at_offset(text, 17), "bit_sr");
    }
}
