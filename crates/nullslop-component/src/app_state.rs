//! Shared application state.
//!
//! [`AppState`] is the single source of truth for what the user sees and how the
//! application is currently behaving. Every component reads from and writes to this
//! shared state.

use nullslop_protocol::{ActiveTab, ChatEntry, Mode};

use crate::chat_input_box::ChatInputBoxState;
use crate::dashboard::DashboardState;
use crate::shutdown_tracker::ShutdownTrackerState;

/// A snapshot of everything the application is doing right now.
#[derive(Debug)]
pub struct AppState {
    /// All messages in the current conversation.
    pub chat_history: Vec<ChatEntry>,

    /// Whether the user is browsing or actively typing.
    pub mode: Mode,

    /// The user's in-progress message and input buffer.
    pub chat_input: ChatInputBoxState,

    /// Bookkeeping for which extensions are still running during shutdown.
    pub shutdown_tracker: ShutdownTrackerState,

    /// Extension dashboard — tracks registered extensions and their status.
    pub dashboard: DashboardState,

    /// The currently active tab.
    pub active_tab: ActiveTab,

    /// Set to `true` when the user has requested to quit.
    pub should_quit: bool,
}

impl AppState {
    /// Create a new `AppState` with no history, normal mode, and empty input.
    #[must_use]
    pub fn new() -> Self {
        Self {
            chat_history: Vec::new(),
            mode: Mode::Normal,
            chat_input: ChatInputBoxState::new(),
            shutdown_tracker: ShutdownTrackerState::new(),
            dashboard: DashboardState::new(),
            active_tab: ActiveTab::Chat,
            should_quit: false,
        }
    }

    /// Append a message to the conversation and return its position.
    pub fn push_entry(&mut self, entry: ChatEntry) -> usize {
        let index = self.chat_history.len();
        self.chat_history.push(entry);
        index
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_data_has_empty_history() {
        let data = AppState::new();
        assert!(data.chat_history.is_empty());
    }

    #[test]
    fn push_entry_adds_to_history() {
        let mut data = AppState::new();
        let entry = ChatEntry::user("hello");
        let index = data.push_entry(entry);
        assert_eq!(index, 0);
        assert_eq!(data.chat_history.len(), 1);
    }

    #[test]
    fn default_mode_is_normal() {
        let data = AppState::new();
        assert_eq!(data.mode, Mode::Normal);
    }

    #[test]
    fn default_chat_input_is_empty() {
        let data = AppState::new();
        assert!(data.chat_input.is_empty());
    }

    #[test]
    fn default_should_quit_is_false() {
        let data = AppState::new();
        assert!(!data.should_quit);
    }
}
