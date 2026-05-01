//! Protocol-level LLM message types.
//!
//! [`LlmMessage`] is a serializable representation of conversation turns,
//! decoupled from the `llm` crate's `ChatMessage`. Used in command payloads
//! that cross the bus boundary.

use serde::{Deserialize, Serialize};

/// Role of a participant in an LLM conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LlmRole {
    /// The user/human participant.
    User,
    /// The AI assistant participant.
    Assistant,
}

/// A single message in an LLM conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmMessage {
    /// The role of who sent this message.
    pub role: LlmRole,
    /// The text content of the message.
    pub content: String,
}
