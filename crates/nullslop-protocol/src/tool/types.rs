//! Tool calling types — definitions, calls, and results.

use serde::{Deserialize, Serialize};

/// A tool definition that describes a tool the LLM can invoke.
///
/// Actors register these at startup via [`RegisterTools`](super::RegisterTools).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolDefinition {
    /// The unique name of the tool (e.g., "`file_read`").
    pub name: String,
    /// Human-readable description of what the tool does.
    pub description: String,
    /// JSON Schema describing the tool's input parameters.
    pub parameters: serde_json::Value,
}

/// A tool call requested by the LLM during a streaming response.
///
/// Contains the function name and JSON arguments the LLM wants to invoke.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCall {
    /// Unique identifier for this tool call (assigned by the LLM provider).
    pub id: String,
    /// The name of the function to call.
    pub name: String,
    /// The arguments as a JSON string.
    pub arguments: String,
}

/// The result of executing a tool call.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolResult {
    /// The ID of the tool call this result is for.
    pub tool_call_id: String,
    /// The name of the tool that was executed.
    pub name: String,
    /// The output content.
    pub content: String,
    /// Whether execution succeeded.
    pub success: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_definition_roundtrip() {
        // Given a tool definition.
        let def = ToolDefinition {
            name: "echo".into(),
            description: "Echoes input".into(),
            parameters: serde_json::json!({"type": "object", "properties": {}}),
        };

        // When serialized and deserialized.
        let json = serde_json::to_string(&def).expect("serialize");
        let back: ToolDefinition = serde_json::from_str(&json).expect("deserialize");

        // Then it matches the original.
        assert_eq!(back, def);
    }

    #[test]
    fn tool_call_roundtrip() {
        // Given a tool call.
        let call = ToolCall {
            id: "call_123".into(),
            name: "echo".into(),
            arguments: r#"{"input":"hi"}"#.into(),
        };

        // When serialized and deserialized.
        let json = serde_json::to_string(&call).expect("serialize");
        let back: ToolCall = serde_json::from_str(&json).expect("deserialize");

        // Then it matches the original.
        assert_eq!(back, call);
    }

    #[test]
    fn tool_result_roundtrip() {
        // Given a tool result.
        let result = ToolResult {
            tool_call_id: "call_123".into(),
            name: "echo".into(),
            content: "hi".into(),
            success: true,
        };

        // When serialized and deserialized.
        let json = serde_json::to_string(&result).expect("serialize");
        let back: ToolResult = serde_json::from_str(&json).expect("deserialize");

        // Then it matches the original.
        assert_eq!(back, result);
    }

    #[test]
    fn tool_result_equality() {
        // Given two identical tool results.
        let a = ToolResult {
            tool_call_id: "call_1".into(),
            name: "echo".into(),
            content: "ok".into(),
            success: true,
        };
        let b = ToolResult {
            tool_call_id: "call_1".into(),
            name: "echo".into(),
            content: "ok".into(),
            success: true,
        };

        // Then they are equal.
        assert_eq!(a, b);
    }
}
