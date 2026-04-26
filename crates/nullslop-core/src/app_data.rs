//! Domain data container shared across threads.

use crate::{ChatEntry, ExtensionRegistry};

/// The domain data of the application.
///
/// This is the data shared across threads via the [`State`](crate::State) wrapper.
/// It contains domain-level state: chat history and extension tracking.
/// Ephemeral UI state (input buffer, scroll offset) lives in `TuiState` in `nullslop-tui`.
/// Application orchestration state (`should_quit`, status) lives in `AppState` in `nullslop-tui`.
#[derive(Debug)]
pub struct AppData {
    /// Chat history entries.
    pub chat_history: Vec<ChatEntry>,
    /// Registered extensions.
    pub extensions: ExtensionRegistry,
}

impl AppData {
    /// Create a new `AppData` with default values.
    #[must_use]
    pub fn new() -> Self {
        Self {
            chat_history: Vec::new(),
            extensions: ExtensionRegistry::new(),
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
}
