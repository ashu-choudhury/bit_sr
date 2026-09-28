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
}
