//! Domain data container for the application.
//!
//! [`AppData`] is the shared state that plugins read and write.
//! It contains domain-level data: chat history, input mode, and
//! the chat input box state. Host-side concerns like extension tracking
//! are managed separately in `nullslop-core`.

use crate::{ChatEntry, ChatInputBoxState, Mode};

/// The domain data of the application.
///
/// This is the data shared across threads via the [`State`](nullslop_core::State) wrapper.
/// It contains domain-level state: chat history, interaction mode, and chat input.
/// Host-side extension tracking lives separately in `nullslop-core`.
#[derive(Debug)]
pub struct AppData {
    /// Chat history entries.
    pub chat_history: Vec<ChatEntry>,
    /// Current interaction mode.
    pub mode: Mode,
    /// Chat input box state.
    pub chat_input: ChatInputBoxState,
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
            chat_input: ChatInputBoxState::new(),
            should_quit: false,
        }
    }

    /// Add a chat entry and return its index.
    pub fn push_entry(&mut self, entry: ChatEntry) -> usize {
        let index = self.chat_history.len();
        self.chat_history.push(entry);
        index
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
    fn default_chat_input_is_empty() {
        // Given a new AppData.
        let data = AppData::new();

        // Then chat input buffer is empty.
        assert!(data.chat_input.input_buffer.is_empty());
    }

    #[test]
    fn default_should_quit_is_false() {
        // Given a new AppData.
        let data = AppData::new();

        // Then should_quit is false.
        assert!(!data.should_quit);
    }
}
