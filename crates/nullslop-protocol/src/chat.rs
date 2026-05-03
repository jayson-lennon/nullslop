//! Conversation data model for the chat log.
//!
//! Each [`ChatEntry`] records a timestamped message from the user,
//! the system, or an actor.

use serde::{Deserialize, Serialize};

/// A single entry in the chat history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatEntry {
    /// When this entry was created.
    pub timestamp: jiff::Timestamp,
    /// What kind of entry this is.
    pub kind: ChatEntryKind,
}

/// The kind of chat entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatEntryKind {
    /// A message typed by the user.
    User(String),
    /// A system-generated message (status updates, etc.).
    System(String),
    /// A response from an AI assistant.
    Assistant(String),
    /// A message from an actor, identified by source name.
    Actor {
        /// The name of the actor that produced this entry.
        source: String,
        /// The message text.
        text: String,
    },
}

impl ChatEntry {
    /// Create a new user chat entry with the current timestamp.
    #[must_use]
    pub fn user<T>(text: T) -> Self
    where
        T: Into<String>,
    {
        Self {
            timestamp: jiff::Timestamp::now(),
            kind: ChatEntryKind::User(text.into()),
        }
    }

    /// Create a new system chat entry with the current timestamp.
    #[must_use]
    pub fn system<T>(text: T) -> Self
    where
        T: Into<String>,
    {
        Self {
            timestamp: jiff::Timestamp::now(),
            kind: ChatEntryKind::System(text.into()),
        }
    }

    /// Create a new assistant chat entry with the current timestamp.
    #[must_use]
    pub fn assistant<T>(text: T) -> Self
    where
        T: Into<String>,
    {
        Self {
            timestamp: jiff::Timestamp::now(),
            kind: ChatEntryKind::Assistant(text.into()),
        }
    }

    /// Create a new actor chat entry with the current timestamp.
    #[must_use]
    pub fn actor<S, T>(source: S, text: T) -> Self
    where
        S: Into<String>,
        T: Into<String>,
    {
        Self {
            timestamp: jiff::Timestamp::now(),
            kind: ChatEntryKind::Actor {
                source: source.into(),
                text: text.into(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_entry_has_user_kind() {
        // Given text "hello".
        let text = "hello";

        // When creating a user entry.
        let entry = ChatEntry::user(text);

        // Then kind is User("hello").
        assert_eq!(entry.kind, ChatEntryKind::User("hello".to_owned()));
    }

    #[test]
    fn system_entry_has_system_kind() {
        // Given text "ready".
        let text = "ready";

        // When creating a system entry.
        let entry = ChatEntry::system(text);

        // Then kind is System("ready").
        assert_eq!(entry.kind, ChatEntryKind::System("ready".to_owned()));
    }

    #[test]
    fn entry_has_timestamp() {
        // Given the current time.
        let before = jiff::Timestamp::now();

        // When creating a user entry.
        let entry = ChatEntry::user("test");

        // Then the timestamp is close to now.
        let after = jiff::Timestamp::now();
        assert!(entry.timestamp >= before);
        assert!(entry.timestamp <= after);
    }

    #[test]
    fn chat_entry_serialization_roundtrip() {
        // Given a ChatEntry.
        let entry = ChatEntry::user("hello");

        // When serialized and deserialized.
        let json = serde_json::to_string(&entry).expect("serialize");
        let back: ChatEntry = serde_json::from_str(&json).expect("deserialize");

        // Then it matches the original.
        assert_eq!(back.kind, entry.kind);
        assert_eq!(back.timestamp, entry.timestamp);
    }

    #[test]
    fn assistant_entry_has_assistant_kind() {
        let text = "hello";
        let entry = ChatEntry::assistant(text);
        assert_eq!(entry.kind, ChatEntryKind::Assistant("hello".to_owned()));
    }

    #[test]
    fn assistant_entry_serialization_roundtrip() {
        let entry = ChatEntry::assistant("hello");
        let json = serde_json::to_string(&entry).expect("serialize");
        let back: ChatEntry = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.kind, entry.kind);
        assert_eq!(back.timestamp, entry.timestamp);
    }

    #[test]
    fn actor_entry_has_actor_kind() {
        // Given source "nullslop-echo" and text "HELLO".
        let source = "nullslop-echo";
        let text = "HELLO";

        // When creating an actor entry.
        let entry = ChatEntry::actor(source, text);

        // Then kind is Actor with correct source and text.
        assert_eq!(
            entry.kind,
            ChatEntryKind::Actor {
                source: "nullslop-echo".to_owned(),
                text: "HELLO".to_owned(),
            }
        );
    }

    #[test]
    fn actor_entry_serialization_roundtrip() {
        // Given an actor ChatEntry.
        let entry = ChatEntry::actor("nullslop-echo", "hello");

        // When serialized and deserialized.
        let json = serde_json::to_string(&entry).expect("serialize");
        let back: ChatEntry = serde_json::from_str(&json).expect("deserialize");

        // Then it matches the original.
        assert_eq!(back.kind, entry.kind);
        assert_eq!(back.timestamp, entry.timestamp);
    }
}
