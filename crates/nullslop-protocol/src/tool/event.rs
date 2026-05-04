//! Tool calling events.

use serde::{Deserialize, Serialize};

use super::types::{ToolDefinition, ToolResult};
use crate::EventMsg;
use crate::SessionId;

/// All tool calls in a batch have completed execution.
///
/// Emitted by the tool orchestrator when every tool call in a batch
/// has finished (success or failure). The LLM actor listens for this
/// to continue the multi-turn tool loop.
#[derive(Debug, Clone, Serialize, Deserialize, EventMsg)]
#[event_msg("tool")]
pub struct ToolBatchCompleted {
    /// The session this batch belongs to.
    pub session_id: SessionId,
    /// The results for each tool call in the batch.
    pub results: Vec<ToolResult>,
}

/// A single tool execution completed.
///
/// Emitted by provider actors after executing a tool.
/// The tool orchestrator aggregates these into a `ToolBatchCompleted`.
#[derive(Debug, Clone, Serialize, Deserialize, EventMsg)]
#[event_msg("tool")]
pub struct ToolExecutionCompleted {
    /// The session this execution belongs to.
    pub session_id: SessionId,
    /// The tool execution result.
    pub result: ToolResult,
}

/// Tools were registered by an actor.
///
/// Emitted after an actor sends `RegisterTools` to confirm registration.
#[derive(Debug, Clone, Serialize, Deserialize, EventMsg)]
#[event_msg("tool")]
pub struct ToolsRegistered {
    /// The name of the actor that registered tools.
    pub provider: String,
    /// The tool definitions that were registered.
    pub definitions: Vec<ToolDefinition>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_batch_completed_roundtrip() {
        // Given a ToolBatchCompleted event.
        let event = ToolBatchCompleted {
            session_id: SessionId::new(),
            results: vec![ToolResult {
                tool_call_id: "call_1".into(),
                name: "echo".into(),
                content: "hi".into(),
                success: true,
            }],
        };

        // When serialized and deserialized.
        let json = serde_json::to_string(&event).expect("serialize");
        let back: ToolBatchCompleted = serde_json::from_str(&json).expect("deserialize");

        // Then it matches.
        assert_eq!(back.results.len(), 1);
    }

    #[test]
    fn tool_batch_completed_type_name() {
        assert_eq!(ToolBatchCompleted::TYPE_NAME, "tool::ToolBatchCompleted");
    }

    #[test]
    fn tool_execution_completed_roundtrip() {
        // Given a ToolExecutionCompleted event.
        let event = ToolExecutionCompleted {
            session_id: SessionId::new(),
            result: ToolResult {
                tool_call_id: "call_1".into(),
                name: "echo".into(),
                content: "ok".into(),
                success: true,
            },
        };

        // When serialized and deserialized.
        let json = serde_json::to_string(&event).expect("serialize");
        let back: ToolExecutionCompleted = serde_json::from_str(&json).expect("deserialize");

        // Then it matches.
        assert_eq!(back.result.content, "ok");
    }

    #[test]
    fn tool_execution_completed_type_name() {
        assert_eq!(
            ToolExecutionCompleted::TYPE_NAME,
            "tool::ToolExecutionCompleted"
        );
    }

    #[test]
    fn tools_registered_roundtrip() {
        // Given a ToolsRegistered event.
        let event = ToolsRegistered {
            provider: "nullslop-echo".into(),
            definitions: vec![ToolDefinition {
                name: "echo".into(),
                description: "Echoes input".into(),
                parameters: serde_json::json!({"type": "object"}),
            }],
        };

        // When serialized and deserialized.
        let json = serde_json::to_string(&event).expect("serialize");
        let back: ToolsRegistered = serde_json::from_str(&json).expect("deserialize");

        // Then it matches.
        assert_eq!(back.provider, "nullslop-echo");
        assert_eq!(back.definitions.len(), 1);
    }

    #[test]
    fn tools_registered_type_name() {
        assert_eq!(ToolsRegistered::TYPE_NAME, "tool::ToolsRegistered");
    }
}
