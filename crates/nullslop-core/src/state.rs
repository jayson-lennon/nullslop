//! Thread-safe application state wrapper.
//!
//! Provides [`State`] as a shared reference to [`AppData`] with
//! read/write guards that don't expose the underlying lock implementation.

use std::sync::Arc;

use parking_lot::RwLock;

use crate::AppData;

/// Thread-safe shared state wrapper.
#[derive(Debug, Clone)]
pub struct State {
    inner: Arc<RwLock<AppData>>,
}

/// Read guard for application data. Does not expose the underlying lock.
pub struct StateReadGuard<'a> {
    inner: parking_lot::RwLockReadGuard<'a, AppData>,
}

/// Write guard for application data. Does not expose the underlying lock.
pub struct StateWriteGuard<'a> {
    inner: parking_lot::RwLockWriteGuard<'a, AppData>,
}

impl State {
    /// Create a new State wrapping the given `AppData`.
    #[must_use]
    pub fn new(data: AppData) -> Self {
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
    type Target = AppData;

    fn deref(&self) -> &AppData {
        &self.inner
    }
}

impl std::ops::Deref for StateWriteGuard<'_> {
    type Target = AppData;

    fn deref(&self) -> &AppData {
        &self.inner
    }
}

impl std::ops::DerefMut for StateWriteGuard<'_> {
    fn deref_mut(&mut self) -> &mut AppData {
        &mut self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChatEntry;

    #[test]
    fn state_read_returns_app_data() {
        // Given a State with a chat entry.
        let mut data = AppData::new();
        data.push_entry(ChatEntry::user("hello"));
        let state = State::new(data);

        // When reading.
        let guard = state.read();

        // Then the entry is visible.
        assert_eq!(guard.chat_history.len(), 1);
    }

    #[test]
    fn state_write_allows_mutation() {
        // Given a State.
        let state = State::new(AppData::new());

        // When writing and pushing an entry.
        {
            let mut guard = state.write();
            guard.push_entry(ChatEntry::user("hello"));
        }

        // Then the entry appears on next read.
        let guard = state.read();
        assert_eq!(guard.chat_history.len(), 1);
    }

    #[test]
    fn state_is_cloneable() {
        // Given a State.
        let state = State::new(AppData::new());

        // When cloning.
        let clone = state.clone();

        // Then both clones point to the same underlying data.
        {
            let mut guard = clone.write();
            guard.push_entry(ChatEntry::user("shared"));
        }
        let guard = state.read();
        assert_eq!(guard.chat_history.len(), 1);
    }

    /// Compile-time check that [`StateReadGuard`] does not expose
    /// `parking_lot::RwLockReadGuard` in its public API.
    ///
    /// The guard type only implements `Deref<Target = AppData>`,
    /// so consumers cannot access the underlying lock.
    #[test]
    fn state_read_guard_hides_lock() {
        // Given a State.
        let state = State::new(AppData::new());

        // When acquiring a read guard.
        let guard = state.read();

        // Then we can only access AppData through Deref.
        let _history = &guard.chat_history;
    }
}
