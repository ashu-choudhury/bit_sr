//! Single-letter quick navigation subsystem for web documents.
//! Provides fast jumping to headings, links, form controls, tables, and landmarks.

use crate::buffer::{BufferLine, TextRun, VirtualBuffer};
use bit_sr_core::input::Key;
use bit_sr_core::roles::Role;
use bit_sr_core::states::State;

/// Target categories for single-letter quick navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuickNavKey {
    /// Jump to any heading (Key: 'H')
    Heading,
    /// Jump to heading at specific level 1-6 (Keys: '1' - '6')
    HeadingLevel(u8),
    /// Jump to any hyperlink (Key: 'K')
    Link,
    /// Jump to unvisited link (Key: 'U')
    UnvisitedLink,
    /// Jump to visited link (Key: 'V')
    VisitedLink,
    /// Jump to any form field (Key: 'F')
    FormField,
    /// Jump to editable text input (Key: 'E')
    EditBox,
    /// Jump to button (Key: 'B')
    Button,
    /// Jump to checkbox (Key: 'X')
    CheckBox,
    /// Jump to radio button (Key: 'R')
    RadioButton,
    /// Jump to combo box / dropdown (Key: 'C')
    ComboBox,
    /// Jump to table or data grid (Key: 'T')
    Table,
    /// Jump to list (Key: 'L')
    List,
    /// Jump to list item (Key: 'I')
    ListItem,
    /// Jump to landmark / banner / navigation / main (Key: 'D')
    Landmark,
    /// Jump to blockquote (Key: 'Q')
    BlockQuote,
    /// Jump to graphic / image (Key: 'G')
    Graphic,
    /// Jump to horizontal separator (Key: 'S')
    Separator,
    /// Jump to embedded frame / iframe (Key: 'M')
    Frame,
}

impl QuickNavKey {
    /// Human-friendly display name of the navigation target category.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Heading => "heading",
            Self::HeadingLevel(1) => "heading level 1",
            Self::HeadingLevel(2) => "heading level 2",
            Self::HeadingLevel(3) => "heading level 3",
            Self::HeadingLevel(4) => "heading level 4",
            Self::HeadingLevel(5) => "heading level 5",
            Self::HeadingLevel(6) => "heading level 6",
            Self::HeadingLevel(_) => "heading",
            Self::Link => "link",
            Self::UnvisitedLink => "unvisited link",
            Self::VisitedLink => "visited link",
            Self::FormField => "form field",
            Self::EditBox => "edit box",
            Self::Button => "button",
            Self::CheckBox => "check box",
            Self::RadioButton => "radio button",
            Self::ComboBox => "combo box",
            Self::Table => "table",
            Self::List => "list",
            Self::ListItem => "list item",
            Self::Landmark => "landmark",
            Self::BlockQuote => "blockquote",
            Self::Graphic => "graphic",
            Self::Separator => "separator",
            Self::Frame => "frame",
        }
    }

    /// Attempts to parse a physical key and modifier into a QuickNavKey and direction.
    /// Returns `Some((key, is_reverse))` if the key matches a quick navigation target.
    pub fn from_key(key: Key, is_shift: bool) -> Option<(Self, bool)> {
        let nav = match key {
            Key::H => Self::Heading,
            Key::Num1 | Key::Numpad1 => Self::HeadingLevel(1),
            Key::Num2 | Key::Numpad2 => Self::HeadingLevel(2),
            Key::Num3 | Key::Numpad3 => Self::HeadingLevel(3),
            Key::Num4 | Key::Numpad4 => Self::HeadingLevel(4),
            Key::Num5 | Key::Numpad5 => Self::HeadingLevel(5),
            Key::Num6 | Key::Numpad6 => Self::HeadingLevel(6),
            Key::K => Self::Link,
            Key::U => Self::UnvisitedLink,
            Key::V => Self::VisitedLink,
            Key::F => Self::FormField,
            Key::E => Self::EditBox,
            Key::B => Self::Button,
            Key::X => Self::CheckBox,
            Key::R => Self::RadioButton,
            Key::C => Self::ComboBox,
            Key::T => Self::Table,
            Key::L => Self::List,
            Key::I => Self::ListItem,
            Key::D => Self::Landmark,
            Key::Q => Self::BlockQuote,
            Key::G => Self::Graphic,
            Key::S => Self::Separator,
            Key::M => Self::Frame,
            _ => return None,
        };
        Some((nav, is_shift))
    }

    /// Returns true if a given `TextRun` matches this quick navigation target.
    pub fn matches_run(&self, run: &TextRun) -> bool {
        match self {
            Self::Heading => run.role == Role::Heading || run.heading_level.is_some(),
            Self::HeadingLevel(level) => {
                run.heading_level == Some(*level)
                    || (run.role == Role::Heading
                        && run.tag.as_deref() == Some(&format!("h{}", level)))
            }
            Self::Link => run.role == Role::Link || run.href.is_some(),
            Self::UnvisitedLink => {
                (run.role == Role::Link || run.href.is_some())
                    && !run.states.contains(State::VISITED)
            }
            Self::VisitedLink => {
                (run.role == Role::Link || run.href.is_some())
                    && run.states.contains(State::VISITED)
            }
            Self::FormField => matches!(
                run.role,
                Role::EditableText
                    | Role::Button
                    | Role::SplitButton
                    | Role::DropDownButton
                    | Role::CheckBox
                    | Role::RadioButton
                    | Role::ComboBox
                    | Role::Slider
                    | Role::SpinButton
                    | Role::Switch
            ),
            Self::EditBox => run.role == Role::EditableText,
            Self::Button => matches!(
                run.role,
                Role::Button | Role::SplitButton | Role::DropDownButton
            ),
            Self::CheckBox => run.role == Role::CheckBox,
            Self::RadioButton => run.role == Role::RadioButton,
            Self::ComboBox => run.role == Role::ComboBox,
            Self::Table => matches!(run.role, Role::Table | Role::DataGrid),
            Self::List => run.role == Role::List,
            Self::ListItem => run.role == Role::ListItem,
            Self::Landmark => matches!(
                run.role,
                Role::Landmark | Role::Header | Role::StatusBar | Role::Section
            ),
            Self::BlockQuote => {
                run.tag.as_deref() == Some("blockquote")
                    || (run.role == Role::Box && run.tag.as_deref() == Some("blockquote"))
            }
            Self::Graphic => run.role == Role::Graphic,
            Self::Separator => run.role == Role::Separator,
            Self::Frame => run.role == Role::Frame,
        }
    }

    /// Returns true if any run on the line matches this target.
    pub fn matches_line(&self, line: &BufferLine) -> bool {
        line.runs.iter().any(|run| self.matches_run(run))
    }
}

/// Quick navigation helper executing forward or backward scans across the VirtualBuffer.
pub struct QuickNav;

impl QuickNav {
    /// Searches forward from the current cursor line for the next line matching the target.
    pub fn find_next(buf: &VirtualBuffer, target: QuickNavKey) -> Option<usize> {
        let start = buf.cursor_line + 1;
        for (i, line) in buf.lines.iter().enumerate().skip(start) {
            if target.matches_line(line) {
                return Some(i);
            }
        }
        None
    }

    /// Searches backward from the current cursor line for the previous line matching the target.
    pub fn find_prev(buf: &VirtualBuffer, target: QuickNavKey) -> Option<usize> {
        if buf.cursor_line == 0 {
            return None;
        }
        let start = buf.cursor_line.saturating_sub(1);
        for i in (0..=start).rev() {
            if let Some(line) = buf.lines.get(i) {
                if target.matches_line(line) {
                    return Some(i);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bit_sr_core::node::NodeId;

    #[test]
    fn test_quick_nav_key_parsing() {
        assert_eq!(
            QuickNavKey::from_key(Key::H, false),
            Some((QuickNavKey::Heading, false))
        );
        assert_eq!(
            QuickNavKey::from_key(Key::H, true),
            Some((QuickNavKey::Heading, true))
        );
        assert_eq!(
            QuickNavKey::from_key(Key::Num2, false),
            Some((QuickNavKey::HeadingLevel(2), false))
        );
        assert_eq!(
            QuickNavKey::from_key(Key::K, false),
            Some((QuickNavKey::Link, false))
        );
        assert_eq!(
            QuickNavKey::from_key(Key::B, false),
            Some((QuickNavKey::Button, false))
        );
        assert_eq!(QuickNavKey::from_key(Key::F1, false), None);
    }

    #[test]
    fn test_quick_nav_forward_and_backward_search() {
        let lines = vec![
            BufferLine::new(0, vec![TextRun::new_heading(NodeId(1), "Introduction", 1)]),
            BufferLine::new(1, vec![TextRun::new_text(NodeId(2), "Some intro text.")]),
            BufferLine::new(2, vec![TextRun::new_link(NodeId(3), "Read docs", None, false)]),
            BufferLine::new(3, vec![TextRun::new_heading(NodeId(4), "Chapter 1", 2)]),
            BufferLine::new(4, vec![TextRun::new_button(NodeId(5), "Next Page")]),
        ];

        let mut buf = VirtualBuffer::with_lines(NodeId(0), lines);
        buf.cursor_line = 0;

        // Search next heading from line 0
        let next_heading = QuickNav::find_next(&buf, QuickNavKey::Heading);
        assert_eq!(next_heading, Some(3)); // Chapter 1

        // Search next button from line 0
        let next_button = QuickNav::find_next(&buf, QuickNavKey::Button);
        assert_eq!(next_button, Some(4)); // Next Page

        // Search next link
        let next_link = QuickNav::find_next(&buf, QuickNavKey::Link);
        assert_eq!(next_link, Some(2)); // Read docs

        // Search previous heading from line 4
        buf.cursor_line = 4;
        let prev_heading = QuickNav::find_prev(&buf, QuickNavKey::Heading);
        assert_eq!(prev_heading, Some(3)); // Chapter 1
    }
}
