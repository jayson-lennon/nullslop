//! Shared application state accessible from any thread.
//!
//! [`State`] wraps [`AppState`] into a single shared reference.
//! Read and write guards provide access without exposing synchronization details.

use std::sync::Arc;

use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use nullslop_component::AppState;

/// Shared application state accessible from any thread.
///
/// Wraps [`AppState`] so readers always see a consistent snapshot.
#[derive(Debug, Clone)]
pub struct State {
    /// The underlying shared, lock-protected application state.
    inner: Arc<RwLock<AppState>>,
}

/// Read guard for application data.
pub struct StateReadGuard<'a> {
    /// The underlying read lock guard.
    inner: RwLockReadGuard<'a, AppState>,
}

/// Write guard for application data.
pub struct StateWriteGuard<'a> {
    /// The underlying write lock guard.
    inner: RwLockWriteGuard<'a, AppState>,
}

impl State {
    /// Create a new State wrapping the given `AppState`.
    #[must_use]
    pub fn new(data: AppState) -> Self {
        Self {
            inner: Arc::new(RwLock::new(data)),
        }
    }

    /// Acquire a read lock on the state.
    pub fn read(&self) -> StateReadGuard<'_> {
        StateReadGuard {
            inner: self.inner.read(),
        }
    }

    /// Acquire a write lock on the state.
    pub fn write(&self) -> StateWriteGuard<'_> {
        StateWriteGuard {
            inner: self.inner.write(),
        }
    }
}

impl std::ops::Deref for StateReadGuard<'_> {
    type Target = AppState;

    fn deref(&self) -> &AppState {
        &self.inner
    }
}

impl std::ops::Deref for StateWriteGuard<'_> {
    type Target = AppState;

    fn deref(&self) -> &AppState {
        &self.inner
    }
}

impl std::ops::DerefMut for StateWriteGuard<'_> {
    fn deref_mut(&mut self) -> &mut AppState {
        &mut self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nullslop_protocol::ChatEntry;

    #[test]
    fn state_read_returns_app_state() {
        // Given a State with a chat entry.
        let mut data = AppState::new();
        data.active_session_mut()
            .push_entry(ChatEntry::user("hello"));
        let state = State::new(data);

        // When reading.
        let guard = state.read();

        // Then the entry is visible.
        assert_eq!(guard.active_session().history().len(), 1);
    }

    #[test]
    fn state_write_allows_mutation() {
        // Given a State.
        let state = State::new(AppState::new());

        // When writing and pushing an entry.
        {
            let mut guard = state.write();
            guard
                .active_session_mut()
                .push_entry(ChatEntry::user("hello"));
        }

        // Then the entry appears on next read.
        let guard = state.read();
        assert_eq!(guard.active_session().history().len(), 1);
    }

    #[test]
    fn state_is_cloneable() {
        // Given a State.
        let state = State::new(AppState::new());

        // When cloning.
        let clone = state.clone();

        // Then both clones point to the same underlying data.
        {
            let mut guard = clone.write();
            guard
                .active_session_mut()
                .push_entry(ChatEntry::user("shared"));
        }
        let guard = state.read();
        assert_eq!(guard.active_session().history().len(), 1);
    }
}
