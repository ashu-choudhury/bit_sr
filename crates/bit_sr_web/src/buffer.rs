//! Virtual Buffer data models and linear cursor tracking for web documents.

use bit_sr_core::node::NodeId;
use bit_sr_core::node::Rect;
use bit_sr_core::roles::Role;
use bit_sr_core::states::State;

/// Current active navigation mode inside a web document or WebView.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NavigationMode {
    /// Virtual cursor intercepts arrow keys and single-letter navigation for reading.
    #[default]
    Browse,
    /// Raw keystrokes pass directly to web content or active form controls for typing.
    Focus,
}

impl NavigationMode {
    /// Returns the human-readable display label.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Browse => "Browse mode",
            Self::Focus => "Focus mode",
        }
    }

    /// Toggles between Browse and Focus mode.
    pub fn toggle(&self) -> Self {
        match self {
            Self::Browse => Self::Focus,
            Self::Focus => Self::Browse,
        }
    }
}

/// A contiguous segment of text within a virtual buffer line with associated semantics.
#[derive(Debug, Clone, PartialEq)]
pub struct TextRun {
    /// Spoken or displayed text for this run.
    pub text: String,
    /// Originating accessible node identifier.
    pub node_id: NodeId,
    /// Semantic accessible role.
    pub role: Role,
    /// State bitflags.
    pub states: State,
    /// HTML tag name if available (e.g. "h1", "a", "button", "input").
    pub tag: Option<String>,
    /// Target URI for hyperlinks.
    pub href: Option<String>,
    /// Heading level (1 to 6) if this run represents a heading.
    pub heading_level: Option<u8>,
    /// Whether this run is clickable / interactive.
    pub is_clickable: bool,
    /// Screen coordinates of this run.
    pub bounds: Option<Rect>,
}

impl TextRun {
    /// Creates a simple static text run.
    pub fn new_text(node_id: NodeId, text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            node_id,
            role: Role::StaticText,
            states: State::empty(),
            tag: None,
            href: None,
            heading_level: None,
            is_clickable: false,
            bounds: None,
        }
    }

    /// Creates a heading text run.
    pub fn new_heading(node_id: NodeId, text: impl Into<String>, level: u8) -> Self {
        Self {
            text: text.into(),
            node_id,
            role: Role::Heading,
            states: State::empty(),
            tag: Some(format!("h{}", level)),
            href: None,
            heading_level: Some(level),
            is_clickable: false,
            bounds: None,
        }
    }

    /// Creates a link text run.
    pub fn new_link(node_id: NodeId, text: impl Into<String>, href: Option<String>, visited: bool) -> Self {
        let mut states = State::LINKED;
        if visited {
            states |= State::VISITED;
        }
        Self {
            text: text.into(),
            node_id,
            role: Role::Link,
            states,
            tag: Some("a".to_string()),
            href,
            heading_level: None,
            is_clickable: true,
            bounds: None,
        }
    }

    /// Creates a button text run.
    pub fn new_button(node_id: NodeId, text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            node_id,
            role: Role::Button,
            states: State::CLICKABLE,
            tag: Some("button".to_string()),
            href: None,
            heading_level: None,
            is_clickable: true,
            bounds: None,
        }
    }

    /// Creates an input/edit text run.
    pub fn new_edit(node_id: NodeId, label: impl Into<String>, value: Option<&str>) -> Self {
        let val_suffix = value.map(|v| format!(": {}", v)).unwrap_or_default();
        Self {
            text: format!("{}{}", label.into(), val_suffix),
            node_id,
            role: Role::EditableText,
            states: State::EDITABLE | State::FOCUSABLE,
            tag: Some("input".to_string()),
            href: None,
            heading_level: None,
            is_clickable: true,
            bounds: None,
        }
    }
}

/// A single linearized line within the virtual document buffer.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BufferLine {
    /// 0-indexed line number in the virtual buffer.
    pub line_number: usize,
    /// Ordered sequence of text runs spanning this line.
    pub runs: Vec<TextRun>,
}

impl BufferLine {
    /// Creates a new buffer line.
    pub fn new(line_number: usize, runs: Vec<TextRun>) -> Self {
        Self { line_number, runs }
    }

    /// Assembles the raw text of this line.
    pub fn text(&self) -> String {
        let mut full = String::new();
        for (i, run) in self.runs.iter().enumerate() {
            if i > 0 && !full.ends_with(' ') && !run.text.starts_with(' ') {
                full.push(' ');
            }
            full.push_str(&run.text);
        }
        full
    }

    /// Returns true if this line contains no printable characters.
    pub fn is_empty(&self) -> bool {
        self.runs.iter().all(|r| r.text.trim().is_empty())
    }

    /// Returns the primary role of this line (e.g. if the line is a heading, returns Role::Heading).
    pub fn primary_role(&self) -> Option<Role> {
        for run in &self.runs {
            if run.role != Role::StaticText && run.role != Role::Unknown {
                return Some(run.role);
            }
        }
        self.runs.first().map(|r| r.role)
    }

    /// Returns the primary node ID of this line.
    pub fn primary_node_id(&self) -> Option<NodeId> {
        self.runs.first().map(|r| r.node_id)
    }

    /// Formats a complete spoken announcement for this line including semantic prefixes.
    pub fn spoken_text(&self) -> String {
        if self.is_empty() {
            return String::new();
        }

        // If the entire line is a single semantic element, give it dedicated presentation
        if self.runs.len() == 1 {
            let run = &self.runs[0];
            return match run.role {
                Role::Heading => {
                    let level_str = run.heading_level.map(|l| format!("heading level {} ", l)).unwrap_or_else(|| "heading ".to_string());
                    format!("{}{}", level_str, run.text.trim())
                }
                Role::Link => {
                    let prefix = if run.states.contains(State::VISITED) { "visited link " } else { "link " };
                    format!("{}{}", prefix, run.text.trim())
                }
                Role::Button => format!("{}, button", run.text.trim()),
                Role::CheckBox => {
                    let state_str = if run.states.contains(State::CHECKED) { "checked" } else { "not checked" };
                    format!("{}, check box {}", run.text.trim(), state_str)
                }
                Role::RadioButton => {
                    let state_str = if run.states.contains(State::CHECKED) { "selected" } else { "not selected" };
                    format!("{}, radio button {}", run.text.trim(), state_str)
                }
                Role::EditableText => format!("{}, edit", run.text.trim()),
                Role::ComboBox => format!("{}, combo box", run.text.trim()),
                Role::Graphic => format!("{}, graphic", run.text.trim()),
                _ => run.text.trim().to_string(),
            };
        }

        // For multi-run lines, assemble runs with semantic cues
        let mut parts = Vec::new();
        for run in &self.runs {
            let trimmed = run.text.trim();
            if trimmed.is_empty() {
                continue;
            }
            match run.role {
                Role::Heading => {
                    let level = run.heading_level.map(|l| format!("heading level {} ", l)).unwrap_or_default();
                    parts.push(format!("{}{}", level, trimmed));
                }
                Role::Link => {
                    let prefix = if run.states.contains(State::VISITED) { "visited link " } else { "link " };
                    parts.push(format!("{}{}", prefix, trimmed));
                }
                Role::Button => parts.push(format!("{}, button", trimmed)),
                Role::CheckBox => {
                    let state = if run.states.contains(State::CHECKED) { "checked" } else { "not checked" };
                    parts.push(format!("{}, check box {}", trimmed, state));
                }
                Role::EditableText => parts.push(format!("{}, edit", trimmed)),
                _ => parts.push(trimmed.to_string()),
            }
        }
        parts.join(" ")
    }

    /// Finds the TextRun and relative character offset for a given character index on this line.
    pub fn run_at_offset(&self, char_offset: usize) -> Option<(&TextRun, usize)> {
        let mut accumulated = 0;
        for run in &self.runs {
            let run_len = run.text.chars().count();
            if char_offset < accumulated + run_len {
                return Some((run, char_offset - accumulated));
            }
            accumulated += run_len;
        }
        self.runs.last().map(|r| (r, r.text.chars().count().saturating_sub(1)))
    }
}

/// In-memory linearized document buffer representing a web page or webview.
#[derive(Debug, Clone, Default)]
pub struct VirtualBuffer {
    /// Root document node identifier.
    pub document_node_id: NodeId,
    /// Document window title.
    pub title: Option<String>,
    /// Document URL or URI.
    pub url: Option<String>,
    /// Linearized lines of the buffer.
    pub lines: Vec<BufferLine>,
    /// 0-indexed current line position of the virtual cursor.
    pub cursor_line: usize,
    /// 0-indexed character offset within the current line.
    pub cursor_offset: usize,
    /// Active navigation mode (Browse vs Focus).
    pub mode: NavigationMode,
}

impl VirtualBuffer {
    /// Creates a new empty virtual buffer anchored to a document node.
    pub fn new(document_node_id: NodeId) -> Self {
        Self {
            document_node_id,
            title: None,
            url: None,
            lines: Vec::new(),
            cursor_line: 0,
            cursor_offset: 0,
            mode: NavigationMode::Browse,
        }
    }

    /// Creates a virtual buffer populated with linearized lines.
    pub fn with_lines(document_node_id: NodeId, lines: Vec<BufferLine>) -> Self {
        Self {
            document_node_id,
            title: None,
            url: None,
            lines,
            cursor_line: 0,
            cursor_offset: 0,
            mode: NavigationMode::Browse,
        }
    }

    /// Total number of lines in the buffer.
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Returns true if the buffer contains no lines.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Returns the currently focused BufferLine.
    pub fn current_line(&self) -> Option<&BufferLine> {
        self.lines.get(self.cursor_line)
    }

    /// Advances the virtual cursor to the next line. Returns None if already at the end.
    pub fn next_line(&mut self) -> Option<&BufferLine> {
        if self.cursor_line + 1 < self.lines.len() {
            self.cursor_line += 1;
            self.cursor_offset = 0;
            self.lines.get(self.cursor_line)
        } else {
            None
        }
    }

    /// Moves the virtual cursor to the previous line. Returns None if already at the beginning.
    pub fn prev_line(&mut self) -> Option<&BufferLine> {
        if self.cursor_line > 0 {
            self.cursor_line -= 1;
            self.cursor_offset = 0;
            self.lines.get(self.cursor_line)
        } else {
            None
        }
    }

    /// Sets the cursor to a specific line index, clamping to bounds.
    pub fn set_cursor_line(&mut self, line: usize) -> bool {
        if line < self.lines.len() {
            self.cursor_line = line;
            self.cursor_offset = 0;
            true
        } else {
            false
        }
    }

    /// Finds the index of the first line containing a run originating from the specified node_id.
    pub fn find_line_by_node_id(&self, node_id: NodeId) -> Option<usize> {
        self.lines.iter().position(|line| {
            line.runs.iter().any(|run| run.node_id == node_id)
        })
    }

    /// Moves cursor to top of the document.
    pub fn doc_start(&mut self) {
        self.cursor_line = 0;
        self.cursor_offset = 0;
    }

    /// Moves cursor to bottom of the document.
    pub fn doc_end(&mut self) {
        if !self.lines.is_empty() {
            self.cursor_line = self.lines.len() - 1;
            self.cursor_offset = 0;
        }
    }

    /// Moves cursor to the start of the current line.
    pub fn line_start(&mut self) {
        self.cursor_offset = 0;
    }

    /// Moves cursor to the end of the current line.
    pub fn line_end(&mut self) {
        if let Some(line) = self.current_line() {
            self.cursor_offset = line.text().chars().count();
        }
    }

    /// Returns the character at the current cursor offset.
    pub fn current_character(&self) -> Option<char> {
        let line = self.current_line()?;
        let text = line.text();
        text.chars().nth(self.cursor_offset)
    }

    /// Advances the cursor to the next character on this line, wrapping to next line if needed.
    pub fn next_character(&mut self) -> Option<char> {
        if let Some(line) = self.current_line() {
            let char_count = line.text().chars().count();
            if self.cursor_offset + 1 < char_count {
                self.cursor_offset += 1;
                return self.current_character();
            }
        }

        // Try wrapping to next line
        if self.next_line().is_some() {
            self.cursor_offset = 0;
            self.current_character()
        } else {
            None
        }
    }

    /// Moves the cursor to the previous character, wrapping to previous line if needed.
    pub fn prev_character(&mut self) -> Option<char> {
        if self.cursor_offset > 0 {
            self.cursor_offset -= 1;
            return self.current_character();
        }

        // Wrap to end of previous line
        if self.prev_line().is_some() {
            if let Some(line) = self.current_line() {
                let count = line.text().chars().count();
                self.cursor_offset = count.saturating_sub(1);
                return self.current_character();
            }
        }
        None
    }

    /// Returns the current word at the cursor.
    pub fn current_word(&self) -> Option<String> {
        let line = self.current_line()?;
        let text = line.text();
        let words: Vec<&str> = text.split_whitespace().collect();
        if words.is_empty() {
            return None;
        }

        let mut offset = 0;
        for word in words {
            let w_len = word.chars().count();
            if self.cursor_offset >= offset && self.cursor_offset <= offset + w_len {
                return Some(word.to_string());
            }
            offset += w_len + 1;
        }
        None
    }

    /// Advances the cursor to the next word.
    pub fn next_word(&mut self) -> Option<String> {
        if let Some(line) = self.current_line() {
            let text = line.text();
            let mut char_idx = 0;
            let mut found_space = false;
            for (idx, c) in text.chars().enumerate() {
                if idx <= self.cursor_offset {
                    char_idx = idx;
                    continue;
                }
                if c.is_whitespace() {
                    found_space = true;
                } else if found_space {
                    self.cursor_offset = idx;
                    return self.current_word();
                }
                char_idx = idx;
            }
            let _ = char_idx;
        }

        // Wrap to first word of next line
        if self.next_line().is_some() {
            self.cursor_offset = 0;
            self.current_word()
        } else {
            None
        }
    }

    /// Moves cursor to the previous word.
    pub fn prev_word(&mut self) -> Option<String> {
        if self.cursor_offset > 0 {
            if let Some(line) = self.current_line() {
                let text = line.text();
                let chars: Vec<char> = text.chars().collect();
                let mut target = self.cursor_offset.saturating_sub(1);
                // Skip trailing spaces
                while target > 0 && chars.get(target).map(|c| c.is_whitespace()).unwrap_or(false) {
                    target -= 1;
                }
                // Skip word letters back to word boundary
                while target > 0 && chars.get(target - 1).map(|c| !c.is_whitespace()).unwrap_or(false) {
                    target -= 1;
                }
                self.cursor_offset = target;
                return self.current_word();
            }
        }

        // Wrap to previous line end
        if self.prev_line().is_some() {
            if let Some(line) = self.current_line() {
                let count = line.text().chars().count();
                self.cursor_offset = count.saturating_sub(1);
                return self.current_word();
            }
        }
        None
    }

    /// Returns the currently focused TextRun under the virtual cursor.
    pub fn current_run(&self) -> Option<&TextRun> {
        let line = self.current_line()?;
        line.run_at_offset(self.cursor_offset).map(|(r, _)| r)
    }

    /// Returns the node ID under the virtual cursor.
    pub fn current_node_id(&self) -> Option<NodeId> {
        self.current_run().map(|r| r.node_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigation_mode_toggle() {
        let mode = NavigationMode::Browse;
        assert_eq!(mode.toggle(), NavigationMode::Focus);
        assert_eq!(mode.toggle().toggle(), NavigationMode::Browse);
    }

    #[test]
    fn test_buffer_line_text_and_spoken_output() {
        let node1 = NodeId(101);
        let node2 = NodeId(102);

        let line = BufferLine::new(
            0,
            vec![
                TextRun::new_heading(node1, "Welcome to the Web", 1),
            ],
        );

        assert_eq!(line.text(), "Welcome to the Web");
        assert_eq!(line.spoken_text(), "heading level 1 Welcome to the Web");
        assert_eq!(line.primary_role(), Some(Role::Heading));
        assert_eq!(line.primary_node_id(), Some(node1));

        let link_line = BufferLine::new(
            1,
            vec![
                TextRun::new_link(node2, "Click here", Some("https://example.com".into()), false),
            ],
        );
        assert_eq!(link_line.spoken_text(), "link Click here");
    }

    #[test]
    fn test_virtual_buffer_line_navigation() {
        let node1 = NodeId(1);
        let node2 = NodeId(2);
        let node3 = NodeId(3);

        let lines = vec![
            BufferLine::new(0, vec![TextRun::new_heading(node1, "Heading 1", 1)]),
            BufferLine::new(1, vec![TextRun::new_text(node2, "First paragraph of content.")]),
            BufferLine::new(2, vec![TextRun::new_button(node3, "Submit")]),
        ];

        let mut buf = VirtualBuffer::with_lines(NodeId(0), lines);
        assert_eq!(buf.line_count(), 3);
        assert_eq!(buf.cursor_line, 0);

        // Advance line
        assert!(buf.next_line().is_some());
        assert_eq!(buf.cursor_line, 1);
        assert_eq!(buf.current_line().unwrap().text(), "First paragraph of content.");

        // Advance to end
        assert!(buf.next_line().is_some());
        assert_eq!(buf.cursor_line, 2);
        assert!(buf.next_line().is_none()); // clamped

        // Move back
        assert!(buf.prev_line().is_some());
        assert_eq!(buf.cursor_line, 1);

        // Document boundary jumps
        buf.doc_start();
        assert_eq!(buf.cursor_line, 0);
        buf.doc_end();
        assert_eq!(buf.cursor_line, 2);
    }

    #[test]
    fn test_find_line_by_node_id() {
        let node1 = NodeId(10);
        let node2 = NodeId(20);
        let node3 = NodeId(30);

        let lines = vec![
            BufferLine::new(0, vec![TextRun::new_heading(node1, "Title", 1)]),
            BufferLine::new(1, vec![TextRun::new_text(node2, "Body text")]),
            BufferLine::new(2, vec![TextRun::new_button(node3, "OK")]),
        ];

        let buf = VirtualBuffer::with_lines(NodeId(1), lines);
        assert_eq!(buf.find_line_by_node_id(node1), Some(0));
        assert_eq!(buf.find_line_by_node_id(node2), Some(1));
        assert_eq!(buf.find_line_by_node_id(node3), Some(2));
        assert_eq!(buf.find_line_by_node_id(NodeId(999)), None);
    }
}
