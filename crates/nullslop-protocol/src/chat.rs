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
    /// A tool call requested by the LLM.
    ToolCall {
        /// Unique ID assigned by the LLM provider.
        id: String,
        /// The function name.
        name: String,
        /// The JSON arguments string.
        arguments: String,
    },
    /// An error that should be prominently displayed to the user.
    Error(String),
    /// The result of executing a tool call.
    ToolResult {
        /// The ID of the tool call this result is for.
        id: String,
        /// The function name.
        name: String,
        /// The output content.
        content: String,
        /// Whether execution succeeded.
        success: bool,
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

    /// Create a new error chat entry with the current timestamp.
    #[must_use]
    pub fn error<T>(text: T) -> Self
    where
        T: Into<String>,
    {
        Self {
            timestamp: jiff::Timestamp::now(),
            kind: ChatEntryKind::Error(text.into()),
        }
    }

    /// Create a new tool call entry with the current timestamp.
    #[must_use]
    pub fn tool_call(
        id: impl Into<String>,
        name: impl Into<String>,
        arguments: impl Into<String>,
    ) -> Self {
        Self {
            timestamp: jiff::Timestamp::now(),
            kind: ChatEntryKind::ToolCall {
                id: id.into(),
                name: name.into(),
                arguments: arguments.into(),
            },
        }
    }

    /// Create a new tool result entry with the current timestamp.
    #[must_use]
    pub fn tool_result(
        id: impl Into<String>,
        name: impl Into<String>,
        content: impl Into<String>,
        success: bool,
    ) -> Self {
        Self {
            timestamp: jiff::Timestamp::now(),
            kind: ChatEntryKind::ToolResult {
                id: id.into(),
                name: name.into(),
                content: content.into(),
                success,
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

    #[test]
    fn tool_call_entry_has_tool_call_kind() {
        // Given tool call details.
        let id = "call_123";
        let name = "echo";
        let arguments = r#"{"input":"hi"}"#;

        // When creating a tool call entry.
        let entry = ChatEntry::tool_call(id, name, arguments);

        // Then kind is ToolCall with correct fields.
        assert_eq!(
            entry.kind,
            ChatEntryKind::ToolCall {
                id: "call_123".to_owned(),
                name: "echo".to_owned(),
                arguments: r#"{"input":"hi"}"#.to_owned(),
            }
        );
    }

    #[test]
    fn tool_result_entry_has_tool_result_kind() {
        // Given tool result details.
        let id = "call_123";
        let name = "echo";
        let content = "hi";

        // When creating a tool result entry.
        let entry = ChatEntry::tool_result(id, name, content, true);

        // Then kind is ToolResult with correct fields.
        assert_eq!(
            entry.kind,
            ChatEntryKind::ToolResult {
                id: "call_123".to_owned(),
                name: "echo".to_owned(),
                content: "hi".to_owned(),
                success: true,
            }
        );
    }

    #[test]
    fn tool_call_entry_serialization_roundtrip() {
        // Given a tool call ChatEntry.
        let entry = ChatEntry::tool_call("call_1", "echo", r#"{"input":"hi"}"#);

        // When serialized and deserialized.
        let json = serde_json::to_string(&entry).expect("serialize");
        let back: ChatEntry = serde_json::from_str(&json).expect("deserialize");

        // Then it matches the original.
        assert_eq!(back.kind, entry.kind);
        assert_eq!(back.timestamp, entry.timestamp);
    }

    #[test]
    fn tool_result_entry_serialization_roundtrip() {
        // Given a tool result ChatEntry.
        let entry = ChatEntry::tool_result("call_1", "echo", "hi", true);

        // When serialized and deserialized.
        let json = serde_json::to_string(&entry).expect("serialize");
        let back: ChatEntry = serde_json::from_str(&json).expect("deserialize");

        // Then it matches the original.
        assert_eq!(back.kind, entry.kind);
        assert_eq!(back.timestamp, entry.timestamp);
    }

    #[test]
    fn error_entry_has_error_kind() {
        // Given text "something went wrong".
        let text = "something went wrong";

        // When creating an error entry.
        let entry = ChatEntry::error(text);

        // Then kind is Error("something went wrong").
        assert_eq!(
            entry.kind,
            ChatEntryKind::Error("something went wrong".to_owned())
        );
    }

    #[test]
    fn error_entry_serialization_roundtrip() {
        // Given an error ChatEntry.
        let entry = ChatEntry::error("something broke");

        // When serialized and deserialized.
        let json = serde_json::to_string(&entry).expect("serialize");
        let back: ChatEntry = serde_json::from_str(&json).expect("deserialize");

        // Then it matches the original.
        assert_eq!(back.kind, entry.kind);
        assert_eq!(back.timestamp, entry.timestamp);
    }
}
