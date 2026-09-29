//! Unified Text Navigation and Caret Traversal Traits.
//! Platform-agnostic interfaces for reading lines, words, and characters at the caret.

/// Granular text unit for caret reading and navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextUnit {
    Character,
    Word,
    Line,
    Paragraph,
    Document,
}

/// Abstract trait implemented by platform drivers (Windows UIA/Win32, Linux AT-SPI2)
/// allowing the screen reader engine to query text at the caret.
pub trait TextProvider: Send + Sync {
    /// Queries text at the current caret position for the given text unit.
    fn get_text_at_caret(&self, unit: TextUnit) -> Option<String>;

    /// Queries the currently selected text, if any.
    fn get_selected_text(&self) -> Option<String> {
        None
    }
}
