//! Session identity types.
//!
//! A [`SessionId`] uniquely identifies a chat session. It is generated
//! using UUID v4 and stored as an opaque string.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A unique identifier for a chat session.
///
/// Generated using UUID v4, stored as an opaque string.
/// Derives equality and hashing so it can be used as a `HashMap` key.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(String);

impl SessionId {
    /// Generate a new unique session ID using UUID v4.
    #[must_use]
    pub fn new() -> Self {
        Self(format!("s-{}", Uuid::new_v4()))
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_id_new_generates_unique_ids() {
        // Given nothing.
        // When generating two session IDs.
        let a = SessionId::new();
        let b = SessionId::new();

        // Then they are different.
        assert_ne!(a, b);
    }

    #[test]
    fn session_id_serialization_roundtrip() {
        // Given a session ID.
        let id = SessionId::new();

        // When serializing and deserializing.
        let json = serde_json::to_string(&id).expect("serialize");
        let back: SessionId = serde_json::from_str(&json).expect("deserialize");

        // Then it roundtrips correctly.
        assert_eq!(id, back);
    }

    #[test]
    fn session_id_starts_with_prefix() {
        // Given a new session ID.
        let id = SessionId::new();

        // When inspecting the string representation.
        // Note: we can't access the inner String directly, so we check serialization.
        let json = serde_json::to_string(&id).expect("serialize");

        // Then the serialized form starts with "s-".
        assert!(json.contains("s-"));
    }
}
