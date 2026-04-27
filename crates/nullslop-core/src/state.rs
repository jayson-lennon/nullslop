//! Thread-safe application state wrapper.
//!
//! Provides [`State`] as a shared reference to [`AppData`] and
//! [`ExtensionRegistry`] with read/write guards that don't expose
//! the underlying lock implementation.

use std::sync::Arc;

use parking_lot::RwLock;

use crate::extension::ExtensionRegistry;
use nullslop_protocol::AppData;

/// Internal state combining domain data and extension registry.
#[derive(Debug)]
struct CoreState {
    /// Domain data (from protocol).
    data: AppData,
    /// Extension registry (host-side concern).
    extensions: ExtensionRegistry,
}

/// Thread-safe shared state wrapper.
///
/// Wraps [`AppData`] from `nullslop-protocol` and the host-side
/// [`ExtensionRegistry`] in a single [`RwLock`] for consistent snapshots.
#[derive(Debug, Clone)]
pub struct State {
    inner: Arc<RwLock<CoreState>>,
}

/// Read guard for application data. Does not expose the underlying lock.
pub struct StateReadGuard<'a> {
    inner: parking_lot::RwLockReadGuard<'a, CoreState>,
}

/// Write guard for application data. Does not expose the underlying lock.
pub struct StateWriteGuard<'a> {
    inner: parking_lot::RwLockWriteGuard<'a, CoreState>,
}

impl State {
    /// Create a new State wrapping the given `AppData`.
    #[must_use]
    pub fn new(data: AppData) -> Self {
        Self {
            inner: Arc::new(RwLock::new(CoreState {
                data,
                extensions: ExtensionRegistry::new(),
            })),
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

impl StateReadGuard<'_> {
    /// Returns a reference to the extension registry.
    #[must_use]
    pub fn extensions(&self) -> &ExtensionRegistry {
        &self.inner.extensions
    }
}

impl StateWriteGuard<'_> {
    /// Returns a reference to the extension registry.
    #[must_use]
    pub fn extensions(&self) -> &ExtensionRegistry {
        &self.inner.extensions
    }

    /// Returns a mutable reference to the extension registry.
    #[must_use]
    pub fn extensions_mut(&mut self) -> &mut ExtensionRegistry {
        &mut self.inner.extensions
    }
}

impl std::ops::Deref for StateReadGuard<'_> {
    type Target = AppData;

    fn deref(&self) -> &AppData {
        &self.inner.data
    }
}

impl std::ops::Deref for StateWriteGuard<'_> {
    type Target = AppData;

    fn deref(&self) -> &AppData {
        &self.inner.data
    }
}

impl std::ops::DerefMut for StateWriteGuard<'_> {
    fn deref_mut(&mut self) -> &mut AppData {
        &mut self.inner.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nullslop_protocol::ChatEntry;

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

    #[test]
    fn state_extensions_are_accessible() {
        // Given a State.
        let state = State::new(AppData::new());

        // When reading extensions.
        let guard = state.read();

        // Then extensions are accessible.
        assert!(guard.extensions().extensions().is_empty());
    }

    #[test]
    fn state_extensions_are_mutable() {
        // Given a State.
        let state = State::new(AppData::new());

        // When registering an extension.
        {
            let mut guard = state.write();
            guard.extensions_mut().register(crate::RegisteredExtension {
                name: "test".to_string(),
                commands: vec!["echo".to_string()],
                subscriptions: vec![],
            });
        }

        // Then the extension is visible on next read.
        let guard = state.read();
        assert_eq!(guard.extensions().extensions().len(), 1);
    }
}
