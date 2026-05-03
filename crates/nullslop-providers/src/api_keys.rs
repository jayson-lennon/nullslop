//! Resolved API keys — populated once at application startup.
//!
//! [`ApiKeys`] maps environment variable names to their resolved values.
//! The application reads all needed env vars at startup and stores them here.
//! No other part of the system touches the environment.

use std::collections::HashMap;

/// Resolved API keys, keyed by environment variable name.
///
/// Built once at application startup by reading the relevant env vars.
/// Passed to [`ProviderRegistry`](crate::ProviderRegistry) for availability
/// checks and factory creation.
#[derive(Debug, Clone, Default)]
pub struct ApiKeys {
    /// Map of env var name → resolved key value.
    keys: HashMap<String, String>,
}

impl ApiKeys {
    /// Creates an empty key store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a resolved key.
    pub fn insert(&mut self, env_var: String, value: String) {
        self.keys.insert(env_var, value);
    }

    /// Looks up a key by its env var name.
    ///
    /// Returns `None` if the key was not resolved at startup.
    #[must_use]
    pub fn get<K>(&self, env_var: K) -> Option<&str>
    where
        K: AsRef<str>,
    {
        self.keys.get(env_var.as_ref()).map(String::as_str)
    }

    /// Returns `true` if a non-empty key exists for the given env var name.
    #[must_use]
    pub fn is_set<K>(&self, env_var: K) -> bool
    where
        K: AsRef<str>,
    {
        self.keys
            .get(env_var.as_ref())
            .is_some_and(|v| !v.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_set_returns_true_for_non_empty_key() {
        // Given a store with a key.
        let mut keys = ApiKeys::new();
        keys.insert("MY_KEY".to_owned(), "sk-secret".to_owned());

        // When checking if set.
        // Then it returns true.
        assert!(keys.is_set("MY_KEY"));
    }

    #[test]
    fn is_set_returns_false_for_empty_key() {
        // Given a store with an empty key.
        let mut keys = ApiKeys::new();
        keys.insert("MY_KEY".to_owned(), String::new());

        // When checking if set.
        // Then it returns false.
        assert!(!keys.is_set("MY_KEY"));
    }

    #[test]
    fn is_set_returns_false_for_missing_key() {
        // Given an empty store.
        let keys = ApiKeys::new();

        // When checking a nonexistent key.
        assert!(!keys.is_set("NONEXISTENT"));
    }

    #[test]
    fn get_returns_value_when_present() {
        // Given a store with a key.
        let mut keys = ApiKeys::new();
        keys.insert("MY_KEY".to_owned(), "sk-secret".to_owned());

        // When getting the value.
        assert_eq!(keys.get("MY_KEY"), Some("sk-secret"));
    }

    #[test]
    fn get_returns_none_when_absent() {
        // Given an empty store.
        let keys = ApiKeys::new();

        // When getting a nonexistent key.
        assert_eq!(keys.get("NOPE"), None);
    }
}
