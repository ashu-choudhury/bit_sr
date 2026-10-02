//! Unified Text Navigation and Caret Traversal Traits.
//! Platform-agnostic interfaces for reading lines, words, and characters at the caret,
//! plus the universal Review Cursor engine for independent text inspection.

/// Granular text unit for caret reading and navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextUnit {
    Character,
    Word,
    Line,
    Paragraph,
    Document,
}

/// Abstract trait implemented by platform drivers (Windows UIA/Win32/Console, Linux AT-SPI2)
/// allowing the screen reader engine to query text at the caret and full documents.
pub trait TextProvider: Send + Sync {
    /// Queries text at the current caret position for the given text unit.
    fn get_text_at_caret(&self, unit: TextUnit) -> Option<String>;

    /// Queries the currently selected text, if any.
    fn get_selected_text(&self) -> Option<String> {
        None
    }

    /// Retrieves full document or visible terminal screen text for review cursor navigation.
    fn get_document_text(&self) -> Option<String> {
        None
    }

    /// Retrieves insertion point / caret character offset within the document text.
    fn get_caret_offset(&self) -> Option<usize> {
        None
    }
}

/// Boundary conditions reached during review cursor traversal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewBoundary {
    Top,
    Bottom,
    Left,
    Right,
}

/// Universal Review Cursor for independent, non-caret text inspection.
/// Operates on console buffers, documents, or UI object text,
/// tracking line, word, and character review positions.
#[derive(Debug, Clone, Default)]
pub struct ReviewCursor {
    text: String,
    lines: Vec<String>,
    current_line: usize,
    current_col: usize,
}

impl ReviewCursor {
    /// Creates a new empty review cursor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Populates text into the review cursor and sets initial position.
    pub fn set_text(&mut self, text: String, initial_char_offset: Option<usize>) {
        self.text = text;
        self.lines.clear();

        if self.text.is_empty() {
            self.lines.push(String::new());
            self.current_line = 0;
            self.current_col = 0;
            return;
        }

        for line in self.text.split('\n') {
            let clean = line.strip_suffix('\r').unwrap_or(line);
            self.lines.push(clean.to_string());
        }

        if self.lines.is_empty() {
            self.lines.push(String::new());
        }

        if let Some(target_offset) = initial_char_offset {
            let mut acc = 0;
            let mut resolved_line = 0;
            let mut resolved_col = 0;

            for (idx, line) in self.lines.iter().enumerate() {
                let char_len = line.chars().count();
                if target_offset <= acc + char_len {
                    resolved_line = idx;
                    resolved_col = target_offset.saturating_sub(acc);
                    break;
                }
                acc += char_len + 1; // +1 for the newline
                resolved_line = idx;
                resolved_col = char_len;
            }

            self.current_line = resolved_line.min(self.lines.len().saturating_sub(1));
            self.current_col = resolved_col.min(self.current_line_char_count());
        } else {
            self.current_line = 0;
            self.current_col = 0;
        }
    }

    /// Updates the underlying text while preserving current line and column positions if possible.
    pub fn update_text(&mut self, text: String) {
        let saved_line = self.current_line;
        let saved_col = self.current_col;
        self.lines = text
            .split('\n')
            .map(|l| l.trim_end_matches('\r').to_string())
            .collect();
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        self.current_line = saved_line.min(self.lines.len().saturating_sub(1));
        self.current_col = saved_col.min(self.current_line_char_count());
    }

    /// Number of lines currently loaded.
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// 0-based index of the line under review.
    pub fn current_line_index(&self) -> usize {
        self.current_line
    }

    /// 0-based character column within the current line.
    pub fn current_col_index(&self) -> usize {
        self.current_col
    }

    /// Returns the character count of the current line.
    fn current_line_char_count(&self) -> usize {
        self.lines
            .get(self.current_line)
            .map(|l| l.chars().count())
            .unwrap_or(0)
    }

    /// Returns the full text under the current line.
    pub fn current_line(&self) -> &str {
        self.lines
            .get(self.current_line)
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    /// Moves the review cursor to the previous line and returns it.
    /// Returns `Err(ReviewBoundary::Top)` if already at the top line.
    pub fn previous_line(&mut self) -> Result<&str, ReviewBoundary> {
        if self.current_line == 0 {
            return Err(ReviewBoundary::Top);
        }
        self.current_line -= 1;
        self.current_col = 0;
        Ok(self.current_line())
    }

    /// Moves the review cursor to the next line and returns it.
    /// Returns `Err(ReviewBoundary::Bottom)` if already at the bottom line.
    pub fn next_line(&mut self) -> Result<&str, ReviewBoundary> {
        if self.current_line + 1 >= self.lines.len() {
            return Err(ReviewBoundary::Bottom);
        }
        self.current_line += 1;
        self.current_col = 0;
        Ok(self.current_line())
    }

    /// Moves the review cursor to the first line (top).
    pub fn top(&mut self) -> &str {
        self.current_line = 0;
        self.current_col = 0;
        self.current_line()
    }

    /// Moves the review cursor to the last line (bottom).
    pub fn bottom(&mut self) -> &str {
        if !self.lines.is_empty() {
            self.current_line = self.lines.len() - 1;
        }
        self.current_col = 0;
        self.current_line()
    }

    /// Returns the word situated under the review cursor.
    pub fn current_word(&self) -> &str {
        let line = self.current_line();
        if line.is_empty() {
            return "";
        }

        let words = Self::extract_words(line);
        if words.is_empty() {
            return "";
        }

        for (start, end) in &words {
            if self.current_col >= *start && self.current_col < *end {
                return &line[Self::char_to_byte_range(line, *start, *end)];
            }
        }

        // If cursor is between words, return next word or last word
        for (start, end) in &words {
            if *start >= self.current_col {
                return &line[Self::char_to_byte_range(line, *start, *end)];
            }
        }

        let (start, end) = words.last().copied().unwrap_or((0, 0));
        &line[Self::char_to_byte_range(line, start, end)]
    }

    /// Moves the review cursor to the previous word.
    /// Traverses up lines if at the start of a line.
    pub fn previous_word(&mut self) -> Result<String, ReviewBoundary> {
        // Try on current line first
        let current_target = {
            let line = self.current_line();
            let words = Self::extract_words(line);
            words
                .into_iter()
                .rev()
                .find(|(start, _)| *start < self.current_col)
        };

        if let Some((start, end)) = current_target {
            self.current_col = start;
            let line = self.current_line();
            return Ok(line[Self::char_to_byte_range(line, start, end)].to_string());
        }

        // Traverse previous lines until a word is found
        let mut probe_line = self.current_line;
        while probe_line > 0 {
            probe_line -= 1;
            let prev_text = &self.lines[probe_line];
            let prev_words = Self::extract_words(prev_text);
            if let Some((start, end)) = prev_words.last().copied() {
                self.current_line = probe_line;
                self.current_col = start;
                let text = &self.lines[self.current_line];
                return Ok(text[Self::char_to_byte_range(text, start, end)].to_string());
            }
        }

        Err(ReviewBoundary::Top)
    }

    /// Moves the review cursor to the next word.
    /// Traverses down lines if at the end of a line.
    pub fn next_word(&mut self) -> Result<String, ReviewBoundary> {
        // Try on current line first
        let current_target = {
            let line = self.current_line();
            let words = Self::extract_words(line);
            words
                .into_iter()
                .find(|(start, _)| *start > self.current_col)
        };

        if let Some((start, end)) = current_target {
            self.current_col = start;
            let line = self.current_line();
            return Ok(line[Self::char_to_byte_range(line, start, end)].to_string());
        }

        // Traverse next lines until a word is found
        let mut probe_line = self.current_line;
        while probe_line + 1 < self.lines.len() {
            probe_line += 1;
            let next_text = &self.lines[probe_line];
            let next_words = Self::extract_words(next_text);
            if let Some((start, end)) = next_words.first().copied() {
                self.current_line = probe_line;
                self.current_col = start;
                let text = &self.lines[self.current_line];
                return Ok(text[Self::char_to_byte_range(text, start, end)].to_string());
            }
        }

        Err(ReviewBoundary::Bottom)
    }

    /// Returns the character situated under the review cursor.
    pub fn current_character(&self) -> Option<char> {
        let line = self.current_line();
        line.chars().nth(self.current_col)
    }

    /// Moves the review cursor to the previous character on the current line.
    /// Returns `Err(ReviewBoundary::Left)` if already at the left edge of the line.
    pub fn previous_character(&mut self) -> Result<char, ReviewBoundary> {
        if self.current_col == 0 {
            return Err(ReviewBoundary::Left);
        }
        self.current_col -= 1;
        self.current_character().ok_or(ReviewBoundary::Left)
    }

    /// Moves the review cursor to the next character on the current line.
    /// Returns `Err(ReviewBoundary::Right)` if already at the right edge of the line.
    pub fn next_character(&mut self) -> Result<char, ReviewBoundary> {
        let total_chars = self.current_line_char_count();
        if self.current_col + 1 >= total_chars {
            return Err(ReviewBoundary::Right);
        }
        self.current_col += 1;
        self.current_character().ok_or(ReviewBoundary::Right)
    }

    /// Moves the review cursor to the first character of the current line.
    pub fn start_of_line(&mut self) -> Option<char> {
        self.current_col = 0;
        self.current_character()
    }

    /// Moves the review cursor to the last character of the current line.
    pub fn end_of_line(&mut self) -> Option<char> {
        let count = self.current_line_char_count();
        if count > 0 {
            self.current_col = count - 1;
        } else {
            self.current_col = 0;
        }
        self.current_character()
    }

    /// Helper extracting start/end character offsets for words in a line.
    fn extract_words(line: &str) -> Vec<(usize, usize)> {
        let mut words = Vec::new();
        let mut in_word = false;
        let mut start = 0;

        for (idx, ch) in line.chars().enumerate() {
            if ch.is_alphanumeric() || ch == '_' {
                if !in_word {
                    in_word = true;
                    start = idx;
                }
            } else if in_word {
                in_word = false;
                words.push((start, idx));
            }
        }

        if in_word {
            words.push((start, line.chars().count()));
        }

        words
    }

    /// Converts char index range to byte range for string slicing.
    fn char_to_byte_range(s: &str, char_start: usize, char_end: usize) -> std::ops::Range<usize> {
        let mut byte_start = s.len();
        let mut byte_end = s.len();
        let mut count = 0;

        for (byte_idx, _) in s.char_indices() {
            if count == char_start {
                byte_start = byte_idx;
            }
            if count == char_end {
                byte_end = byte_idx;
                break;
            }
            count += 1;
        }

        if count < char_end {
            byte_end = s.len();
        }

        byte_start.min(s.len())..byte_end.min(s.len())
    }
}

/// Returns the standard NATO phonetic alphabet name for a letter,
/// or a descriptive representation for punctuation/symbols.
pub fn nato_phonetic(c: char) -> &'static str {
    match c {
        'a' | 'A' => "Alpha",
        'b' | 'B' => "Bravo",
        'c' | 'C' => "Charlie",
        'd' | 'D' => "Delta",
        'e' | 'E' => "Echo",
        'f' | 'F' => "Foxtrot",
        'g' | 'G' => "Golf",
        'h' | 'H' => "Hotel",
        'i' | 'I' => "India",
        'j' | 'J' => "Juliett",
        'k' | 'K' => "Kilo",
        'l' | 'L' => "Lima",
        'm' | 'M' => "Mike",
        'n' | 'N' => "November",
        'o' | 'O' => "Oscar",
        'p' | 'P' => "Papa",
        'q' | 'Q' => "Quebec",
        'r' | 'R' => "Romeo",
        's' | 'S' => "Sierra",
        't' | 'T' => "Tango",
        'u' | 'U' => "Uniform",
        'v' | 'V' => "Victor",
        'w' | 'W' => "Whiskey",
        'x' | 'X' => "X-ray",
        'y' | 'Y' => "Yankee",
        'z' | 'Z' => "Zulu",

        '0' => "Zero",
        '1' => "One",
        '2' => "Two",
        '3' => "Three",
        '4' => "Four",
        '5' => "Five",
        '6' => "Six",
        '7' => "Seven",
        '8' => "Eight",
        '9' => "Nine",

        ' ' => "Space",
        '.' => "Period",
        ',' => "Comma",
        ';' => "Semicolon",
        ':' => "Colon",
        '!' => "Exclamation mark",
        '?' => "Question mark",
        '\'' => "Apostrophe",
        '"' => "Quote",
        '/' => "Slash",
        '\\' => "Backslash",
        '-' => "Hyphen",
        '_' => "Underscore",
        '+' => "Plus",
        '=' => "Equals",
        '<' => "Less than",
        '>' => "Greater than",
        '(' => "Left paren",
        ')' => "Right paren",
        '[' => "Left bracket",
        ']' => "Right bracket",
        '{' => "Left brace",
        '}' => "Right brace",
        '@' => "At",
        '#' => "Hash",
        '$' => "Dollar",
        '%' => "Percent",
        '^' => "Caret",
        '&' => "Ampersand",
        '*' => "Asterisk",
        '~' => "Tilde",
        '`' => "Grave",
        '|' => "Pipe",
        '\t' => "Tab",
        '\n' | '\r' => "Line feed",

        _ => "Unknown character",
    }
}

/// Returns character decimal ordinal and hexadecimal representation (e.g. "97, 0x61").
pub fn char_ordinal_description(c: char) -> String {
    let code = c as u32;
    format!("{}, 0x{:02X}", code, code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_review_cursor_line_navigation() {
        let mut cursor = ReviewCursor::new();
        cursor.set_text("First line\nSecond line\nThird line".to_string(), None);

        assert_eq!(cursor.line_count(), 3);
        assert_eq!(cursor.current_line(), "First line");

        // Previous on first line -> Top boundary
        assert_eq!(cursor.previous_line(), Err(ReviewBoundary::Top));

        // Next line -> Second line
        assert_eq!(cursor.next_line(), Ok("Second line"));
        assert_eq!(cursor.current_line_index(), 1);

        // Next line -> Third line
        assert_eq!(cursor.next_line(), Ok("Third line"));

        // Next on last line -> Bottom boundary
        assert_eq!(cursor.next_line(), Err(ReviewBoundary::Bottom));

        // Top and bottom shortcuts
        assert_eq!(cursor.top(), "First line");
        assert_eq!(cursor.bottom(), "Third line");
    }

    #[test]
    fn test_review_cursor_word_and_char_navigation() {
        let mut cursor = ReviewCursor::new();
        cursor.set_text("hello world\nfoo bar".to_string(), None);

        assert_eq!(cursor.current_word(), "hello");
        assert_eq!(cursor.current_character(), Some('h'));

        // Next char
        assert_eq!(cursor.next_character(), Ok('e'));
        assert_eq!(cursor.current_character(), Some('e'));

        // Start of line
        assert_eq!(cursor.start_of_line(), Some('h'));
        assert_eq!(cursor.previous_character(), Err(ReviewBoundary::Left));

        // End of line
        assert_eq!(cursor.end_of_line(), Some('d'));
        assert_eq!(cursor.next_character(), Err(ReviewBoundary::Right));

        // Word navigation across lines
        assert_eq!(cursor.next_word(), Ok("foo".to_string()));
        assert_eq!(cursor.current_word(), "foo");
        assert_eq!(cursor.next_word(), Ok("bar".to_string()));
        assert_eq!(cursor.next_word(), Err(ReviewBoundary::Bottom));

        assert_eq!(cursor.previous_word(), Ok("foo".to_string()));
        assert_eq!(cursor.previous_word(), Ok("world".to_string()));
        assert_eq!(cursor.previous_word(), Ok("hello".to_string()));
        assert_eq!(cursor.previous_word(), Err(ReviewBoundary::Top));
    }

    #[test]
    fn test_nato_and_ordinal() {
        assert_eq!(nato_phonetic('a'), "Alpha");
        assert_eq!(nato_phonetic('Z'), "Zulu");
        assert_eq!(nato_phonetic('7'), "Seven");
        assert_eq!(nato_phonetic(' '), "Space");

        assert_eq!(char_ordinal_description('a'), "97, 0x61");
        assert_eq!(char_ordinal_description('A'), "65, 0x41");
    }

    #[test]
    fn test_review_cursor_update_text() {
        let mut cursor = ReviewCursor::new();
        cursor.set_text("line 1\nline 2".to_string(), Some(7)); // start at line 2 ('l')
        assert_eq!(cursor.current_line_index(), 1);
        assert_eq!(cursor.current_line(), "line 2");

        // Update with new line appended (e.g., terminal output)
        cursor.update_text("line 1\nline 2\nline 3".to_string());
        assert_eq!(cursor.current_line_index(), 1);
        assert_eq!(cursor.current_line(), "line 2");
        assert_eq!(cursor.next_line(), Ok("line 3"));
    }
}
