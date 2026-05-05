//! Conversion between protocol types and llm crate types.
//!
//! All functions are `pub(crate)` — these are internal implementation details
//! used by service implementations. No llm crate types leak through the
//! public [`LlmService`](crate::LlmService) trait.

use llm::chat::{ChatMessage, ChatRole, FunctionTool, MessageType, StreamChunk, Tool};
use llm::{FunctionCall, ToolCall as LlmToolCall};
use nullslop_protocol::LlmMessage;
use nullslop_protocol::tool::{ToolCall, ToolDefinition};

use crate::StreamEvent;

/// Convert protocol messages to llm crate messages.
///
/// Maps each [`LlmMessage`] variant to the appropriate [`ChatMessage`]:
/// - `System` → system message
/// - `User` → user text message
/// - `Assistant` with tool_calls → assistant tool-use message
/// - `Assistant` without tool_calls → assistant text message
/// - `Tool` → user tool-result message (llm crate convention)
pub(crate) fn messages_to_llm(messages: &[LlmMessage]) -> Vec<ChatMessage> {
    messages.iter().map(message_to_llm).collect()
}

/// Convert protocol tool definitions to llm crate tools.
pub(crate) fn tool_definitions_to_llm(defs: &[ToolDefinition]) -> Vec<Tool> {
    defs.iter().map(tool_definition_to_llm).collect()
}

/// Convert a protocol tool call to an llm crate tool call.
pub(crate) fn tool_call_to_llm(tc: &ToolCall) -> LlmToolCall {
    LlmToolCall {
        id: tc.id.clone(),
        call_type: "function".to_string(),
        function: FunctionCall {
            name: tc.name.clone(),
            arguments: tc.arguments.clone(),
        },
    }
}

/// Convert an llm crate tool call to a protocol tool call.
pub(crate) fn llm_tool_call_to_protocol(tc: &LlmToolCall) -> ToolCall {
    ToolCall {
        id: tc.id.clone(),
        name: tc.function.name.clone(),
        arguments: tc.function.arguments.clone(),
    }
}

/// Convert an llm crate stream chunk to our stream event.
pub(crate) fn stream_chunk_to_event(chunk: StreamChunk) -> StreamEvent {
    match chunk {
        StreamChunk::Text(text) => StreamEvent::Text(text),
        StreamChunk::ToolUseStart { index, id, name } => {
            StreamEvent::ToolUseStart { index, id, name }
        }
        StreamChunk::ToolUseInputDelta {
            index,
            partial_json,
        } => StreamEvent::ToolUseInputDelta {
            index,
            partial_json,
        },
        StreamChunk::ToolUseComplete { index, tool_call } => StreamEvent::ToolUseComplete {
            index,
            tool_call: llm_tool_call_to_protocol(&tool_call),
        },
        StreamChunk::Done { stop_reason } => StreamEvent::Done { stop_reason },
    }
}

/// Convert a single protocol message to an llm crate message.
fn message_to_llm(msg: &LlmMessage) -> ChatMessage {
    match msg {
        LlmMessage::System { content } => {
            // The llm crate doesn't have a System ChatRole, so we convert to
            // a User message. The prompt assembly layer is responsible for
            // placing system prompts correctly. When a proper system message
            // API is available, this can be updated.
            ChatMessage::user().content(content).build()
        }
        LlmMessage::User { content } => ChatMessage::user().content(content).build(),
        LlmMessage::Assistant {
            content,
            tool_calls: None,
        } => ChatMessage::assistant().content(content).build(),
        LlmMessage::Assistant {
            content,
            tool_calls: Some(calls),
        } => {
            let llm_calls: Vec<LlmToolCall> = calls.iter().map(tool_call_to_llm).collect();
            ChatMessage::assistant()
                .content(content)
                .tool_use(llm_calls)
                .build()
        }
        LlmMessage::Tool {
            tool_call_id,
            name,
            content,
        } => {
            // llm crate convention: tool results use User role with ToolResult message type,
            // carrying the tool call ID in a synthetic LlmToolCall.
            let synthetic_call = LlmToolCall {
                id: tool_call_id.clone(),
                call_type: "function".to_string(),
                function: FunctionCall {
                    name: name.clone(),
                    arguments: content.clone(),
                },
            };
            ChatMessage {
                role: ChatRole::User,
                message_type: MessageType::ToolResult(vec![synthetic_call]),
                content: content.clone(),
            }
        }
    }
}

/// Convert a single protocol tool definition to an llm crate tool.
fn tool_definition_to_llm(def: &ToolDefinition) -> Tool {
    Tool {
        tool_type: "function".to_string(),
        function: FunctionTool {
            name: def.name.clone(),
            description: def.description.clone(),
            parameters: def.parameters.clone(),
        },
        cache_control: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_user_message_to_chat_message() {
        // Given a protocol user message.
        let msg = LlmMessage::User {
            content: "hello".to_owned(),
        };

        // When converting to ChatMessage.
        let chat_msg = message_to_llm(&msg);

        // Then content and role are correct.
        assert_eq!(chat_msg.content, "hello");
        assert_eq!(chat_msg.role, ChatRole::User);
    }

    #[test]
    fn convert_assistant_message_to_chat_message() {
        // Given a protocol assistant message without tool calls.
        let msg = LlmMessage::Assistant {
            content: "hi there".to_owned(),
            tool_calls: None,
        };

        // When converting to ChatMessage.
        let chat_msg = message_to_llm(&msg);

        // Then content and role are correct.
        assert_eq!(chat_msg.content, "hi there");
        assert_eq!(chat_msg.role, ChatRole::Assistant);
    }

    #[test]
    fn convert_assistant_message_with_tool_calls_to_chat_message() {
        // Given a protocol assistant message with tool calls.
        let msg = LlmMessage::Assistant {
            content: String::new(),
            tool_calls: Some(vec![ToolCall {
                id: "call_1".to_string(),
                name: "echo".to_string(),
                arguments: r#"{"input":"hi"}"#.to_string(),
            }]),
        };

        // When converting to ChatMessage.
        let chat_msg = message_to_llm(&msg);

        // Then role is assistant and message type is ToolUse.
        assert_eq!(chat_msg.role, ChatRole::Assistant);
        assert!(matches!(chat_msg.message_type, MessageType::ToolUse(_)));
    }

    #[test]
    fn convert_tool_result_message_to_chat_message() {
        // Given a protocol tool result message.
        let msg = LlmMessage::Tool {
            tool_call_id: "call_1".to_string(),
            name: "echo".to_string(),
            content: "result data".to_string(),
        };

        // When converting to ChatMessage.
        let chat_msg = message_to_llm(&msg);

        // Then role is User and message type is ToolResult.
        assert_eq!(chat_msg.role, ChatRole::User);
        assert!(matches!(chat_msg.message_type, MessageType::ToolResult(_)));
    }

    #[test]
    fn messages_to_llm_converts_list() {
        // Given a list of protocol messages.
        let messages = vec![
            LlmMessage::User {
                content: "hello".to_owned(),
            },
            LlmMessage::Assistant {
                content: "hi".to_owned(),
                tool_calls: None,
            },
        ];

        // When converting.
        let result = messages_to_llm(&messages);

        // Then both are converted correctly.
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].role, ChatRole::User);
        assert_eq!(result[1].role, ChatRole::Assistant);
    }

    #[test]
    fn tool_definitions_to_llm_converts_all() {
        // Given tool definitions.
        let defs = vec![
            ToolDefinition {
                name: "echo".to_string(),
                description: "Echoes input".to_string(),
                parameters: serde_json::json!({"type": "object", "properties": {}}),
            },
            ToolDefinition {
                name: "get_time".to_string(),
                description: "Returns current time".to_string(),
                parameters: serde_json::json!({"type": "object", "properties": {}}),
            },
        ];

        // When converting.
        let result = tool_definitions_to_llm(&defs);

        // Then all definitions are converted.
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].function.name, "echo");
        assert_eq!(result[1].function.name, "get_time");
    }

    #[test]
    fn tool_call_to_llm_preserves_fields() {
        // Given a protocol tool call.
        let tc = ToolCall {
            id: "call_123".to_string(),
            name: "echo".to_string(),
            arguments: r#"{"input":"hi"}"#.to_string(),
        };

        // When converting to llm ToolCall.
        let llm_tc = tool_call_to_llm(&tc);

        // Then fields are preserved.
        assert_eq!(llm_tc.id, "call_123");
        assert_eq!(llm_tc.function.name, "echo");
        assert_eq!(llm_tc.function.arguments, r#"{"input":"hi"}"#);
        assert_eq!(llm_tc.call_type, "function");
    }

    #[test]
    fn llm_tool_call_to_protocol_roundtrip() {
        // Given a protocol tool call.
        let original = ToolCall {
            id: "call_abc".to_string(),
            name: "file_read".to_string(),
            arguments: r#"{"path":"/tmp/f"}"#.to_string(),
        };

        // When converting to llm and back.
        let llm_tc = tool_call_to_llm(&original);
        let back = llm_tool_call_to_protocol(&llm_tc);

        // Then the round-trip preserves fields.
        assert_eq!(back.id, original.id);
        assert_eq!(back.name, original.name);
        assert_eq!(back.arguments, original.arguments);
    }

    #[test]
    fn stream_chunk_to_event_text() {
        // Given an llm StreamChunk::Text.
        let chunk = StreamChunk::Text("hello".to_string());

        // When converting.
        let event = stream_chunk_to_event(chunk);

        // Then it produces a StreamEvent::Text.
        assert_eq!(event, StreamEvent::Text("hello".to_string()));
    }

    #[test]
    fn stream_chunk_to_event_tool_use_start() {
        // Given an llm StreamChunk::ToolUseStart.
        let chunk = StreamChunk::ToolUseStart {
            index: 0,
            id: "call_1".to_string(),
            name: "echo".to_string(),
        };

        // When converting.
        let event = stream_chunk_to_event(chunk);

        // Then it produces a StreamEvent::ToolUseStart.
        assert_eq!(
            event,
            StreamEvent::ToolUseStart {
                index: 0,
                id: "call_1".to_string(),
                name: "echo".to_string(),
            }
        );
    }

    #[test]
    fn stream_chunk_to_event_tool_use_complete() {
        // Given an llm StreamChunk::ToolUseComplete.
        let chunk = StreamChunk::ToolUseComplete {
            index: 0,
            tool_call: LlmToolCall {
                id: "call_1".to_string(),
                call_type: "function".to_string(),
                function: FunctionCall {
                    name: "echo".to_string(),
                    arguments: r#"{"input":"hi"}"#.to_string(),
                },
            },
        };

        // When converting.
        let event = stream_chunk_to_event(chunk);

        // Then it produces a StreamEvent::ToolUseComplete.
        assert_eq!(
            event,
            StreamEvent::ToolUseComplete {
                index: 0,
                tool_call: ToolCall {
                    id: "call_1".to_string(),
                    name: "echo".to_string(),
                    arguments: r#"{"input":"hi"}"#.to_string(),
                },
            }
        );
    }

    #[test]
    fn stream_chunk_to_event_done() {
        // Given an llm StreamChunk::Done.
        let chunk = StreamChunk::Done {
            stop_reason: "tool_use".to_string(),
        };

        // When converting.
        let event = stream_chunk_to_event(chunk);

        // Then it produces a StreamEvent::Done.
        assert_eq!(
            event,
            StreamEvent::Done {
                stop_reason: "tool_use".to_string(),
            }
        );
    }
}
