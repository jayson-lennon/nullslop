//! Provider commands.

use serde::{Deserialize, Serialize};

use super::message::LlmMessage;
use crate::CommandMsg;
use crate::SessionId;

/// Switch the active LLM provider.
///
/// Carries the target provider ID. The handler validates it against the registry,
/// swaps the factory, and emits [`ProviderSwitched`](super::ProviderSwitched).
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("provider")]
pub struct ProviderSwitch {
    /// The provider to switch to.
    pub provider_id: String,
}

/// Send a message to the AI provider.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("provider")]
pub struct SendMessage {
    /// The session this message belongs to.
    pub session_id: SessionId,
    /// The message text.
    pub text: String,
}

/// Cancel the active provider stream for a session.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("provider")]
pub struct CancelStream {
    /// The session whose stream should be cancelled.
    pub session_id: SessionId,
}

/// A single token from a streaming LLM response.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("provider")]
pub struct StreamToken {
    /// The session this token belongs to.
    pub session_id: SessionId,
    /// The zero-based index of this token in the stream.
    pub index: usize,
    /// The token text.
    pub token: String,
}

/// Command to send conversation context to the LLM provider.
///
/// Emitted by `LlmRequestHandler` when a user message is submitted.
/// Carries the full conversation history as pre-converted messages.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("provider")]
pub struct SendToLlmProvider {
    /// The session this request belongs to.
    pub session_id: SessionId,
    /// The full conversation history, converted to LLM messages.
    pub messages: Vec<LlmMessage>,
    /// Optional provider override for per-message routing (future).
    /// Currently always `None` — uses the active provider.
    #[serde(default)]
    pub provider_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_to_llm_provider_deserializes_without_provider_id() {
        // Given JSON without the provider_id field (old format).
        let json = r#"{"session_id":"sid-1","messages":[]}"#;

        // When deserializing.
        let cmd: SendToLlmProvider = serde_json::from_str(json).expect("deserialize");

        // Then provider_id is None (backwards compatible).
        assert!(cmd.provider_id.is_none());
    }
}
