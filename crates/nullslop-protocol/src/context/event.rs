//! Event types for prompt assembly.

use serde::{Deserialize, Serialize};

use crate::EventMsg;
use crate::provider::LlmMessage;
use crate::context::PromptStrategyId;
use crate::SessionId;

/// Emitted when a prompt has been assembled and is ready to send.
///
/// The message queue handler receives this event, finishes assembling,
/// and submits the messages to the LLM provider.
#[derive(Debug, Clone, Serialize, Deserialize, EventMsg)]
#[event_msg("context")]
pub struct PromptAssembled {
    /// The session this assembly is for.
    pub session_id: SessionId,
    /// System prompt, if any. Should be prepended as `LlmMessage::System`.
    pub system_prompt: Option<String>,
    /// The assembled messages ready for the LLM.
    pub messages: Vec<LlmMessage>,
}

/// Emitted when a session's prompt assembly strategy has been switched.
///
/// Emitted by the `PromptAssemblyActor` after successfully switching
/// a session to a new strategy.
#[derive(Debug, Clone, Serialize, Deserialize, EventMsg)]
#[event_msg("context")]
pub struct PromptStrategySwitched {
    /// The session whose strategy was switched.
    pub session_id: SessionId,
    /// The new strategy that is now active.
    pub strategy_id: PromptStrategyId,
}

/// Emitted when a strategy's session state has changed and should be persisted.
///
/// The component handler stores the blob in `AppState` for later restoration.
/// The host doesn't interpret the blob — it just stores and restores it.
#[derive(Debug, Clone, Serialize, Deserialize, EventMsg)]
#[event_msg("context")]
pub struct StrategyStateUpdated {
    /// The session whose strategy state changed.
    pub session_id: SessionId,
    /// The strategy the state belongs to.
    pub strategy_id: PromptStrategyId,
    /// The opaque state blob to persist.
    pub blob: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_assembled_serialization_roundtrip() {
        // Given a PromptAssembled event.
        let evt = PromptAssembled {
            session_id: SessionId::new(),
            system_prompt: Some("You are helpful.".to_owned()),
            messages: vec![LlmMessage::User {
                content: "hello".to_owned(),
            }],
        };

        // When serialized and deserialized.
        let json = serde_json::to_string(&evt).expect("serialize");
        let back: PromptAssembled = serde_json::from_str(&json).expect("deserialize");

        // Then fields are preserved.
        assert_eq!(back.system_prompt, Some("You are helpful.".to_owned()));
        assert_eq!(back.messages.len(), 1);
    }

    #[test]
    fn prompt_assembled_has_type_name() {
        assert_eq!(PromptAssembled::TYPE_NAME, "context::PromptAssembled");
    }

    #[test]
    fn prompt_strategy_switched_serialization_roundtrip() {
        // Given a PromptStrategySwitched event.
        let evt = PromptStrategySwitched {
            session_id: SessionId::new(),
            strategy_id: PromptStrategyId::sliding_window(),
        };

        // When serialized and deserialized.
        let json = serde_json::to_string(&evt).expect("serialize");
        let back: PromptStrategySwitched = serde_json::from_str(&json).expect("deserialize");

        // Then fields are preserved.
        assert_eq!(back.strategy_id, PromptStrategyId::sliding_window());
    }

    #[test]
    fn prompt_strategy_switched_has_type_name() {
        assert_eq!(PromptStrategySwitched::TYPE_NAME, "context::PromptStrategySwitched");
    }

    #[test]
    fn strategy_state_updated_serialization_roundtrip() {
        // Given a StrategyStateUpdated event.
        let evt = StrategyStateUpdated {
            session_id: SessionId::new(),
            strategy_id: PromptStrategyId::compaction(),
            blob: serde_json::json!({"compaction_count": 3}),
        };

        // When serialized and deserialized.
        let json = serde_json::to_string(&evt).expect("serialize");
        let back: StrategyStateUpdated = serde_json::from_str(&json).expect("deserialize");

        // Then fields are preserved.
        assert_eq!(back.strategy_id, PromptStrategyId::compaction());
        assert_eq!(back.blob["compaction_count"], 3);
    }

    #[test]
    fn strategy_state_updated_has_type_name() {
        assert_eq!(StrategyStateUpdated::TYPE_NAME, "context::StrategyStateUpdated");
    }
}
