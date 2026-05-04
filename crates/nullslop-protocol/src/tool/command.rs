//! Tool calling commands.

use serde::{Deserialize, Serialize};

use super::types::{ToolCall, ToolDefinition, ToolResult};
use crate::CommandMsg;
use crate::SessionId;

/// Register tools that an actor can execute.
///
/// Sent by actors at startup to declare which tools they provide.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("tool")]
pub struct RegisterTools {
    /// The name of the actor providing these tools.
    pub provider: String,
    /// The tool definitions being registered.
    pub definitions: Vec<ToolDefinition>,
}

/// Request execution of a batch of tool calls for a session.
///
/// Sent by the LLM actor when the LLM produces tool calls.
/// Routed to the tool orchestrator.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("tool")]
pub struct ExecuteToolBatch {
    /// The session requesting tool execution.
    pub session_id: SessionId,
    /// The tool calls to execute.
    pub tool_calls: Vec<ToolCall>,
}

/// Execute a single tool call.
///
/// Sent by the tool orchestrator to the actor that registered the tool.
/// Carries the session ID so the provider actor can include it in its
/// response event.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("tool")]
pub struct ExecuteTool {
    /// The session this execution belongs to.
    pub session_id: SessionId,
    /// The tool call to execute.
    pub tool_call: ToolCall,
}

/// A tool call has started in the LLM stream (name and ID known, arguments pending).
///
/// Emitted by the LLM actor when the backend signals `ToolUseStart`.
/// The chat log creates a placeholder entry for this tool call.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("tool")]
pub struct ToolUseStarted {
    /// The session this tool call belongs to.
    pub session_id: SessionId,
    /// The index of the tool call in the response.
    pub index: usize,
    /// The unique ID for this tool call (assigned by the LLM provider).
    pub id: String,
    /// The name of the tool being called.
    pub name: String,
}

/// A tool call was received from the LLM stream (fully assembled).
///
/// Emitted by the LLM actor when a complete tool call arrives in the stream.
/// The chat log uses this to finalize the tool call entry.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("tool")]
pub struct ToolCallReceived {
    /// The session this tool call belongs to.
    pub session_id: SessionId,
    /// The assembled tool call.
    pub tool_call: ToolCall,
}

/// Streaming update for a tool call being assembled.
///
/// Emitted by the LLM actor as tool call arguments stream in.
/// The chat log uses this to render in-progress tool call arguments.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("tool")]
pub struct ToolCallStreaming {
    /// The session this tool call belongs to.
    pub session_id: SessionId,
    /// The index of the tool call in the response.
    pub index: usize,
    /// Partial JSON string for the tool arguments (accumulated so far).
    pub partial_json: String,
}

/// Push a tool result into the chat log.
///
/// Emitted by the LLM actor after tool execution completes, before
/// re-sending to the LLM.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("tool")]
pub struct PushToolResult {
    /// The session this result belongs to.
    pub session_id: SessionId,
    /// The tool execution result.
    pub result: ToolResult,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_tools_roundtrip() {
        // Given a RegisterTools command.
        let cmd = RegisterTools {
            provider: "nullslop-echo".into(),
            definitions: vec![ToolDefinition {
                name: "echo".into(),
                description: "Echoes input".into(),
                parameters: serde_json::json!({"type": "object"}),
            }],
        };

        // When serialized and deserialized.
        let json = serde_json::to_string(&cmd).expect("serialize");
        let back: RegisterTools = serde_json::from_str(&json).expect("deserialize");

        // Then it matches.
        assert_eq!(back.provider, "nullslop-echo");
        assert_eq!(back.definitions.len(), 1);
    }

    #[test]
    fn register_tools_name() {
        assert_eq!(RegisterTools::NAME, "tool::RegisterTools");
    }

    #[test]
    fn execute_tool_batch_roundtrip() {
        // Given an ExecuteToolBatch command.
        let cmd = ExecuteToolBatch {
            session_id: SessionId::new(),
            tool_calls: vec![ToolCall {
                id: "call_1".into(),
                name: "echo".into(),
                arguments: "{}".into(),
            }],
        };

        // When serialized and deserialized.
        let json = serde_json::to_string(&cmd).expect("serialize");
        let back: ExecuteToolBatch = serde_json::from_str(&json).expect("deserialize");

        // Then it matches.
        assert_eq!(back.tool_calls.len(), 1);
    }

    #[test]
    fn execute_tool_batch_name() {
        assert_eq!(ExecuteToolBatch::NAME, "tool::ExecuteToolBatch");
    }

    #[test]
    fn execute_tool_roundtrip() {
        // Given an ExecuteTool command.
        let cmd = ExecuteTool {
            session_id: SessionId::new(),
            tool_call: ToolCall {
                id: "call_1".into(),
                name: "echo".into(),
                arguments: r#"{"input":"hi"}"#.into(),
            },
        };

        // When serialized and deserialized.
        let json = serde_json::to_string(&cmd).expect("serialize");
        let back: ExecuteTool = serde_json::from_str(&json).expect("deserialize");

        // Then it matches.
        assert_eq!(back.tool_call.name, "echo");
    }

    #[test]
    fn execute_tool_name() {
        assert_eq!(ExecuteTool::NAME, "tool::ExecuteTool");
    }

    #[test]
    fn tool_use_started_roundtrip() {
        // Given a ToolUseStarted command.
        let cmd = ToolUseStarted {
            session_id: SessionId::new(),
            index: 0,
            id: "call_1".into(),
            name: "echo".into(),
        };

        // When serialized and deserialized.
        let json = serde_json::to_string(&cmd).expect("serialize");
        let back: ToolUseStarted = serde_json::from_str(&json).expect("deserialize");

        // Then it matches.
        assert_eq!(back.id, "call_1");
        assert_eq!(back.name, "echo");
    }

    #[test]
    fn tool_use_started_name() {
        assert_eq!(ToolUseStarted::NAME, "tool::ToolUseStarted");
    }

    #[test]
    fn tool_call_received_roundtrip() {
        // Given a ToolCallReceived command.
        let cmd = ToolCallReceived {
            session_id: SessionId::new(),
            tool_call: ToolCall {
                id: "call_1".into(),
                name: "echo".into(),
                arguments: "{}".into(),
            },
        };

        // When serialized and deserialized.
        let json = serde_json::to_string(&cmd).expect("serialize");
        let back: ToolCallReceived = serde_json::from_str(&json).expect("deserialize");

        // Then it matches.
        assert_eq!(back.tool_call.id, "call_1");
    }

    #[test]
    fn tool_call_received_name() {
        assert_eq!(ToolCallReceived::NAME, "tool::ToolCallReceived");
    }

    #[test]
    fn tool_call_streaming_roundtrip() {
        // Given a ToolCallStreaming command.
        let cmd = ToolCallStreaming {
            session_id: SessionId::new(),
            index: 2,
            partial_json: r#"{"input":"he"#.into(),
        };

        // When serialized and deserialized.
        let json = serde_json::to_string(&cmd).expect("serialize");
        let back: ToolCallStreaming = serde_json::from_str(&json).expect("deserialize");

        // Then it matches.
        assert_eq!(back.index, 2);
        assert_eq!(back.partial_json, r#"{"input":"he"#);
    }

    #[test]
    fn tool_call_streaming_name() {
        assert_eq!(ToolCallStreaming::NAME, "tool::ToolCallStreaming");
    }

    #[test]
    fn push_tool_result_roundtrip() {
        // Given a PushToolResult command.
        let cmd = PushToolResult {
            session_id: SessionId::new(),
            result: ToolResult {
                tool_call_id: "call_1".into(),
                name: "echo".into(),
                content: "hi".into(),
                success: true,
            },
        };

        // When serialized and deserialized.
        let json = serde_json::to_string(&cmd).expect("serialize");
        let back: PushToolResult = serde_json::from_str(&json).expect("deserialize");

        // Then it matches.
        assert_eq!(back.result.content, "hi");
        assert!(back.result.success);
    }

    #[test]
    fn push_tool_result_name() {
        assert_eq!(PushToolResult::NAME, "tool::PushToolResult");
    }
}
