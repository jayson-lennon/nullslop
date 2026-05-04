//! Stream events from LLM chat with tool support.
//!
//! [`StreamEvent`] is our own type for streaming LLM responses, decoupled
//! from the `llm` crate's `StreamChunk`. All conversion happens inside
//! provider implementations.

use nullslop_protocol::tool::ToolCall;

/// A streaming event from an LLM chat response.
///
/// Produced by [`LlmService::chat_stream_with_tools`](super::LlmService::chat_stream_with_tools).
/// The stream always ends with a [`Done`](StreamEvent::Done) event.
#[derive(Debug, Clone, PartialEq)]
pub enum StreamEvent {
    /// A text content delta.
    Text(String),
    /// A tool use block started (ID and name known, arguments streaming).
    ToolUseStart {
        /// The index of this content block in the response.
        index: usize,
        /// The unique ID for this tool call (assigned by the LLM provider).
        id: String,
        /// The name of the tool being called.
        name: String,
    },
    /// A partial JSON delta for tool call arguments.
    ToolUseInputDelta {
        /// The index of this content block.
        index: usize,
        /// Partial JSON string for the tool input.
        partial_json: String,
    },
    /// A tool use block completed with an assembled tool call.
    ToolUseComplete {
        /// The index of this content block.
        index: usize,
        /// The complete tool call.
        tool_call: ToolCall,
    },
    /// The stream ended.
    Done {
        /// Why the stream stopped (e.g., "end_turn", "tool_use").
        stop_reason: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_formatting() {
        // Given all StreamEvent variants.
        let text = StreamEvent::Text("hello".to_string());
        let tool_start = StreamEvent::ToolUseStart {
            index: 0,
            id: "call_1".to_string(),
            name: "echo".to_string(),
        };
        let tool_delta = StreamEvent::ToolUseInputDelta {
            index: 0,
            partial_json: r#"{"input":"h"#.to_string(),
        };
        let tool_complete = StreamEvent::ToolUseComplete {
            index: 0,
            tool_call: ToolCall {
                id: "call_1".to_string(),
                name: "echo".to_string(),
                arguments: r#"{"input":"hi"}"#.to_string(),
            },
        };
        let done = StreamEvent::Done {
            stop_reason: "end_turn".to_string(),
        };

        // When formatting with Debug.
        // Then each variant produces a non-empty debug string.
        assert!(!format!("{text:?}").is_empty());
        assert!(!format!("{tool_start:?}").is_empty());
        assert!(!format!("{tool_delta:?}").is_empty());
        assert!(!format!("{tool_complete:?}").is_empty());
        assert!(!format!("{done:?}").is_empty());
    }

    #[test]
    fn partial_eq_all_variants() {
        // Given two identical sets of events.
        let text_a = StreamEvent::Text("hello".to_string());
        let text_b = StreamEvent::Text("hello".to_string());

        let start_a = StreamEvent::ToolUseStart {
            index: 0,
            id: "call_1".to_string(),
            name: "echo".to_string(),
        };
        let start_b = StreamEvent::ToolUseStart {
            index: 0,
            id: "call_1".to_string(),
            name: "echo".to_string(),
        };

        let delta_a = StreamEvent::ToolUseInputDelta {
            index: 0,
            partial_json: r#"{"x"#.to_string(),
        };
        let delta_b = StreamEvent::ToolUseInputDelta {
            index: 0,
            partial_json: r#"{"x"#.to_string(),
        };

        let complete_a = StreamEvent::ToolUseComplete {
            index: 0,
            tool_call: ToolCall {
                id: "call_1".to_string(),
                name: "echo".to_string(),
                arguments: "{}".to_string(),
            },
        };
        let complete_b = StreamEvent::ToolUseComplete {
            index: 0,
            tool_call: ToolCall {
                id: "call_1".to_string(),
                name: "echo".to_string(),
                arguments: "{}".to_string(),
            },
        };

        let done_a = StreamEvent::Done {
            stop_reason: "tool_use".to_string(),
        };
        let done_b = StreamEvent::Done {
            stop_reason: "tool_use".to_string(),
        };

        // Then each pair is equal.
        assert_eq!(text_a, text_b);
        assert_eq!(start_a, start_b);
        assert_eq!(delta_a, delta_b);
        assert_eq!(complete_a, complete_b);
        assert_eq!(done_a, done_b);
    }
}
