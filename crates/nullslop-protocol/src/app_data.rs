//! Domain data container for the application.
//!
//! [`AppData`] is the shared state that plugins read and write.
//! It contains domain-level data: chat history, input mode, and
//! the input buffer. Host-side concerns like extension tracking
//! are managed separately in `nullslop-core`.

use unicode_segmentation::UnicodeSegmentation;

use crate::{ChatEntry, Mode};

/// The domain data of the application.
///
/// This is the data shared across threads via the [`State`](nullslop_core::State) wrapper.
/// It contains domain-level state: chat history, interaction mode, and input buffer.
/// Host-side extension tracking lives separately in `nullslop-core`.
#[derive(Debug)]
pub struct AppData {
    /// Chat history entries.
    pub chat_history: Vec<ChatEntry>,
    /// Current interaction mode.
    pub mode: Mode,
    /// The current text in the input box.
    pub input_buffer: String,
    /// Whether the application should exit.
    pub should_quit: bool,
}

impl AppData {
    /// Create a new `AppData` with default values.
    #[must_use]
    pub fn new() -> Self {
        Self {
            chat_history: Vec::new(),
            mode: Mode::Normal,
            input_buffer: String::new(),
            should_quit: false,
        }
    }

    /// Add a chat entry and return its index.
    pub fn push_entry(&mut self, entry: ChatEntry) -> usize {
        let index = self.chat_history.len();
        self.chat_history.push(entry);
        index
    }

    /// Removes the last grapheme cluster from the input buffer.
    ///
    /// Uses `unicode_segmentation` to handle multi-byte characters correctly.
    pub fn pop_grapheme(&mut self) {
        if let Some((idx, _)) = self.input_buffer.grapheme_indices(true).next_back() {
            self.input_buffer.truncate(idx);
        }
    }
}

impl Default for AppData {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_data_has_empty_history() {
        // Given a new AppData.
        let data = AppData::new();

        // When inspecting chat history.

        // Then chat history is empty.
        assert!(data.chat_history.is_empty());
    }

    #[test]
    fn push_entry_adds_to_history() {
        // Given an AppData.
        let mut data = AppData::new();

        // When pushing an entry.
        let entry = ChatEntry::user("hello");
        let index = data.push_entry(entry);

        // Then history length increases.
        assert_eq!(index, 0);
        assert_eq!(data.chat_history.len(), 1);
    }

    #[test]
    fn default_mode_is_normal() {
        // Given a new AppData.
        let data = AppData::new();

        // Then mode is Normal.
        assert_eq!(data.mode, Mode::Normal);
    }

    #[test]
    fn default_input_buffer_is_empty() {
        // Given a new AppData.
        let data = AppData::new();

        // Then input buffer is empty.
        assert!(data.input_buffer.is_empty());
    }

    #[test]
    fn default_should_quit_is_false() {
        // Given a new AppData.
        let data = AppData::new();

        // Then should_quit is false.
        assert!(!data.should_quit);
    }

    #[test]
    fn pop_grapheme_removes_last() {
        // Given an AppData with "abc" in the buffer.
        let mut data = AppData::new();
        data.input_buffer = "abc".to_string();

        // When popping a grapheme.
        data.pop_grapheme();

        // Then buffer is "ab".
        assert_eq!(data.input_buffer, "ab");
    }

    #[test]
    fn pop_grapheme_handles_unicode() {
        // Given an AppData with "é" in the buffer.
        let mut data = AppData::new();
        data.input_buffer = "é".to_string();

        // When popping a grapheme.
        data.pop_grapheme();

        // Then buffer is empty.
        assert_eq!(data.input_buffer, "");
    }

    #[test]
    fn pop_grapheme_empty_is_noop() {
        // Given an AppData with empty buffer.
        let mut data = AppData::new();

        // When popping a grapheme.
        data.pop_grapheme();

        // Then buffer is still empty.
        assert!(data.input_buffer.is_empty());
    }
}
