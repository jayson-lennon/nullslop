//! Application state container.
//!
//! [`AppState`] is the shared state that components read and write.
//! It contains chat history, input mode, and
//! the chat input box state. Host-side concerns like extension tracking
//! are managed separately in `nullslop-core`.

use std::collections::HashSet;

use nullslop_protocol::{ChatEntry, Mode};

use crate::ChatInputBoxState;

/// Tracks which extensions have started and completed shutdown.
#[derive(Debug, Clone, Default)]
pub struct ShutdownTracker {
    /// Extensions currently tracked as starting/running.
    pending: HashSet<String>,
    /// Whether shutdown has been initiated.
    pub shutdown_active: bool,
}

impl ShutdownTracker {
    /// Creates a new empty tracker.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Marks an extension as starting (tracked for shutdown coordination).
    pub fn track(&mut self, name: &str) {
        self.pending.insert(name.to_string());
    }

    /// Marks an extension as having completed shutdown.
    /// Returns true if the extension was in the pending set.
    pub fn complete(&mut self, name: &str) -> bool {
        self.pending.remove(name)
    }

    /// Returns true when shutdown is active and all tracked extensions have completed.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.shutdown_active && self.pending.is_empty()
    }

    /// Returns the names of extensions still pending.
    #[must_use]
    pub fn pending_names(&self) -> Vec<String> {
        self.pending.iter().cloned().collect()
    }
}

/// The application state.
///
/// This is the state shared across threads via the [`State`](nullslop_core::State) wrapper.
/// It contains chat history, interaction mode, and chat input.
/// Host-side extension tracking lives separately in `nullslop-core`.
#[derive(Debug)]
pub struct AppState {
    /// Chat history entries.
    pub chat_history: Vec<ChatEntry>,
    /// Current interaction mode.
    pub mode: Mode,
    /// Chat input box state.
    pub chat_input: ChatInputBoxState,
    /// Whether the application should exit.
    pub should_quit: bool,
    /// Shutdown coordination tracker.
    pub shutdown_tracker: ShutdownTracker,
}

impl AppState {
    /// Create a new `AppState` with default values.
    #[must_use]
    pub fn new() -> Self {
        Self {
            chat_history: Vec::new(),
            mode: Mode::Normal,
            chat_input: ChatInputBoxState::new(),
            should_quit: false,
            shutdown_tracker: ShutdownTracker::new(),
        }
    }

    /// Add a chat entry and return its index.
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
        assert!(data.chat_input.input_buffer.is_empty());
    }

    #[test]
    fn default_should_quit_is_false() {
        let data = AppState::new();
        assert!(!data.should_quit);
    }
}

#[cfg(test)]
mod shutdown_tracker_tests {
    use super::ShutdownTracker;

    #[test]
    fn track_adds_to_pending() {
        let mut tracker = ShutdownTracker::new();
        tracker.track("ext-a");
        assert_eq!(tracker.pending_names(), vec!["ext-a".to_string()]);
    }

    #[test]
    fn complete_removes_from_pending() {
        let mut tracker = ShutdownTracker::new();
        tracker.track("ext-a");
        let was_tracked = tracker.complete("ext-a");
        assert!(was_tracked);
        assert!(tracker.pending_names().is_empty());
    }

    #[test]
    fn is_complete_false_when_not_active() {
        let tracker = ShutdownTracker::new();
        assert!(!tracker.is_complete());
    }

    #[test]
    fn is_complete_false_when_pending() {
        let mut tracker = ShutdownTracker::new();
        tracker.track("ext-a");
        tracker.shutdown_active = true;
        assert!(!tracker.is_complete());
    }

    #[test]
    fn is_complete_true_when_active_and_empty() {
        let mut tracker = ShutdownTracker::new();
        tracker.shutdown_active = true;
        assert!(tracker.is_complete());
    }

    #[test]
    fn pending_names_returns_pending() {
        let mut tracker = ShutdownTracker::new();
        tracker.track("ext-a");
        tracker.track("ext-b");
        tracker.track("ext-c");
        let mut names = tracker.pending_names();
        names.sort();
        assert_eq!(names, vec!["ext-a", "ext-b", "ext-c"]);
    }
}
