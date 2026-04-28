//! Chat log entry types.
//!
//! Entries can be from the user, the system, or an extension.

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
    /// A message from an extension, identified by source name.
    Extension {
        /// The name of the extension that produced this entry.
        source: String,
        /// The message text.
        text: String,
    },
}

impl ChatEntry {
    /// Create a new user chat entry with the current timestamp.
    #[must_use]
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            timestamp: jiff::Timestamp::now(),
            kind: ChatEntryKind::User(text.into()),
        }
    }

    /// Create a new system chat entry with the current timestamp.
    #[must_use]
    pub fn system(text: impl Into<String>) -> Self {
        Self {
            timestamp: jiff::Timestamp::now(),
            kind: ChatEntryKind::System(text.into()),
        }
    }

    /// Create a new extension chat entry with the current timestamp.
    #[must_use]
    pub fn extension(source: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            timestamp: jiff::Timestamp::now(),
            kind: ChatEntryKind::Extension {
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
        assert_eq!(entry.kind, ChatEntryKind::User("hello".to_string()));
    }

    #[test]
    fn system_entry_has_system_kind() {
        // Given text "ready".
        let text = "ready";

        // When creating a system entry.
        let entry = ChatEntry::system(text);

        // Then kind is System("ready").
        assert_eq!(entry.kind, ChatEntryKind::System("ready".to_string()));
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
    fn extension_entry_has_extension_kind() {
        // Given source "nullslop-echo" and text "HELLO".
        let source = "nullslop-echo";
        let text = "HELLO";

        // When creating an extension entry.
        let entry = ChatEntry::extension(source, text);

        // Then kind is Extension with correct source and text.
        assert_eq!(
            entry.kind,
            ChatEntryKind::Extension {
                source: "nullslop-echo".to_string(),
                text: "HELLO".to_string(),
            }
        );
    }

    #[test]
    fn extension_entry_serialization_roundtrip() {
        // Given an extension ChatEntry.
        let entry = ChatEntry::extension("nullslop-echo", "hello");

        // When serialized and deserialized.
        let json = serde_json::to_string(&entry).expect("serialize");
        let back: ChatEntry = serde_json::from_str(&json).expect("deserialize");

        // Then it matches the original.
        assert_eq!(back.kind, entry.kind);
        assert_eq!(back.timestamp, entry.timestamp);
    }
}
