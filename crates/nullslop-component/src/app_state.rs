//! Shared application state.
//!
//! [`AppState`] is the single source of truth for what the user sees and how the
//! application is currently behaving. Every component reads from and writes to this
//! shared state.

use std::collections::HashMap;

use nullslop_protocol::{ActiveTab, Mode, SessionId};

use crate::chat_input_box::ChatInputBoxState;
use crate::chat_session::ChatSessionState;
use crate::dashboard::DashboardState;
use crate::shutdown_tracker::ShutdownTrackerState;

/// A snapshot of everything the application is doing right now.
#[derive(Debug)]
pub struct AppState {
    /// All chat sessions, keyed by session ID.
    pub sessions: HashMap<SessionId, ChatSessionState>,

    /// The currently active session ID.
    pub active_session: SessionId,

    /// Whether the user is browsing or actively typing.
    pub mode: Mode,

    /// Bookkeeping for which actors are still running during shutdown.
    pub shutdown_tracker: ShutdownTrackerState,

    /// Actor dashboard — tracks registered actors and their status.
    pub dashboard: DashboardState,

    /// The currently active tab.
    pub active_tab: ActiveTab,

    /// Set to `true` when the user has requested to quit.
    pub should_quit: bool,
}

impl AppState {
    /// Create a new `AppState` with one default session, normal mode, and empty input.
    #[must_use]
    pub fn new() -> Self {
        let active_session = SessionId::new();
        let mut sessions = HashMap::new();
        sessions.insert(active_session.clone(), ChatSessionState::new());
        Self {
            sessions,
            active_session,
            mode: Mode::Normal,
            shutdown_tracker: ShutdownTrackerState::new(),
            dashboard: DashboardState::new(),
            active_tab: ActiveTab::Chat,
            should_quit: false,
        }
    }

    /// Read-only access to the active chat session.
    ///
    /// # Panics
    ///
    /// Panics if the active session does not exist in the sessions map.
    /// This should never happen in normal operation.
    #[expect(
        clippy::expect_used,
        reason = "active session invariant guaranteed by construction"
    )]
    pub fn active_session(&self) -> &ChatSessionState {
        self.sessions
            .get(&self.active_session)
            .expect("active session must exist")
    }

    /// Mutable access to the active chat session.
    ///
    /// # Panics
    ///
    /// Panics if the active session does not exist in the sessions map.
    /// This should never happen in normal operation.
    #[expect(
        clippy::expect_used,
        reason = "active session invariant guaranteed by construction"
    )]
    pub fn active_session_mut(&mut self) -> &mut ChatSessionState {
        self.sessions
            .get_mut(&self.active_session)
            .expect("active session must exist")
    }

    /// Read-only access to a session by ID.
    ///
    /// # Panics
    ///
    /// Panics if the given session ID does not exist in the sessions map.
    #[expect(
        clippy::expect_used,
        reason = "session invariant guaranteed by construction"
    )]
    pub fn session(&self, id: &SessionId) -> &ChatSessionState {
        self.sessions.get(id).expect("session must exist")
    }

    /// Mutable access to a session by ID.
    ///
    /// # Panics
    ///
    /// Panics if the given session ID does not exist in the sessions map.
    #[expect(
        clippy::expect_used,
        reason = "session invariant guaranteed by construction"
    )]
    pub fn session_mut(&mut self, id: &SessionId) -> &mut ChatSessionState {
        self.sessions.get_mut(id).expect("session must exist")
    }

    /// Read-only access to the active session's input box.
    ///
    /// Delegates to [`ChatSessionState::chat_input`] on the active session.
    ///
    /// # Panics
    ///
    /// Panics if the active session does not exist in the sessions map.
    pub fn active_chat_input(&self) -> &ChatInputBoxState {
        self.active_session().chat_input()
    }

    /// Mutable access to the active session's input box.
    ///
    /// Delegates to [`ChatSessionState::chat_input_mut`] on the active session.
    ///
    /// # Panics
    ///
    /// Panics if the active session does not exist in the sessions map.
    pub fn active_chat_input_mut(&mut self) -> &mut ChatInputBoxState {
        self.active_session_mut().chat_input_mut()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use nullslop_protocol::ChatEntry;

    use super::*;

    #[test]
    fn push_entry_adds_to_history() {
        // Given a new AppState.
        let mut data = AppState::new();
        let entry = ChatEntry::user("hello");

        // When pushing an entry via the active session.
        let index = data.active_session_mut().push_entry(entry);

        // Then the index is 0 and history has one entry.
        assert_eq!(index, 0);
        assert_eq!(data.active_session().history().len(), 1);
    }
}
