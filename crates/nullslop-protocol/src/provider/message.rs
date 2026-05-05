//! Protocol-level LLM message types.
//!
//! [`LlmMessage`] is a serializable representation of conversation turns,
//! decoupled from the `llm` crate's `ChatMessage`. Used in command payloads
//! that cross the bus boundary.

use serde::{Deserialize, Serialize};

use crate::tool::ToolCall;

/// A single message in an LLM conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum LlmMessage {
    /// A system-level instruction to the LLM.
    System {
        /// The system prompt content.
        content: String,
    },
    /// A message from the user.
    User {
        /// The text content of the message.
        content: String,
    },
    /// A message from the AI assistant.
    Assistant {
        /// The text content of the message.
        content: String,
        /// Tool calls the assistant wants to make, if any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tool_calls: Option<Vec<ToolCall>>,
    },
    /// A tool result message.
    Tool {
        /// The ID of the tool call this result is for.
        tool_call_id: String,
        /// The name of the tool that was executed.
        name: String,
        /// The output content.
        content: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_variant_roundtrip() {
        // Given a User message.
        let msg = LlmMessage::User {
            content: "hello".into(),
        };

        // When serialized and deserialized.
        let json = serde_json::to_string(&msg).expect("serialize");
        let back: LlmMessage = serde_json::from_str(&json).expect("deserialize");

        // Then it matches the original.
        assert_eq!(back, msg);
    }

    #[test]
    fn assistant_variant_without_tool_calls_roundtrip() {
        // Given an Assistant message without tool calls.
        let msg = LlmMessage::Assistant {
            content: "hi".into(),
            tool_calls: None,
        };

        // When serialized and deserialized.
        let json = serde_json::to_string(&msg).expect("serialize");
        let back: LlmMessage = serde_json::from_str(&json).expect("deserialize");

        // Then it matches the original.
        assert_eq!(back, msg);
    }

    #[test]
    fn assistant_variant_with_tool_calls_roundtrip() {
        // Given an Assistant message with tool calls.
        let msg = LlmMessage::Assistant {
            content: String::new(),
            tool_calls: Some(vec![ToolCall {
                id: "call_1".into(),
                name: "echo".into(),
                arguments: "{}".into(),
            }]),
        };

        // When serialized and deserialized.
        let json = serde_json::to_string(&msg).expect("serialize");
        let back: LlmMessage = serde_json::from_str(&json).expect("deserialize");

        // Then it matches the original.
        assert_eq!(back, msg);
    }

    #[test]
    fn tool_variant_roundtrip() {
        // Given a Tool result message.
        let msg = LlmMessage::Tool {
            tool_call_id: "call_1".into(),
            name: "echo".into(),
            content: "hi".into(),
        };

        // When serialized and deserialized.
        let json = serde_json::to_string(&msg).expect("serialize");
        let back: LlmMessage = serde_json::from_str(&json).expect("deserialize");

        // Then it matches the original.
        assert_eq!(back, msg);
    }

    #[test]
    fn backward_compat_user_deserialization() {
        // Given old-format JSON for a user message.
        let json = r#"{"role":"user","content":"hello"}"#;

        // When deserializing.
        let msg: LlmMessage = serde_json::from_str(json).expect("deserialize");

        // Then it produces the expected variant.
        assert_eq!(
            msg,
            LlmMessage::User {
                content: "hello".into()
            }
        );
    }

    #[test]
    fn backward_compat_assistant_deserialization() {
        // Given old-format JSON for an assistant message.
        let json = r#"{"role":"assistant","content":"hi"}"#;

        // When deserializing.
        let msg: LlmMessage = serde_json::from_str(json).expect("deserialize");

        // Then it produces the expected variant with no tool calls.
        assert_eq!(
            msg,
            LlmMessage::Assistant {
                content: "hi".into(),
                tool_calls: None,
            }
        );
    }
}
