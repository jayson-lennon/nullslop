//! Conversion from chat entries to LLM messages.

use super::message::LlmMessage;
use crate::ChatEntry;
use crate::ChatEntryKind;
use crate::tool::ToolCall;

/// Convert chat history entries to LLM messages.
///
/// Includes `User`, `Assistant`, `ToolCall`, and `ToolResult` entries.
/// System and actor entries are skipped since they are not part of the
/// conversation context for the LLM.
///
/// Assistant entries that follow a `ToolCall` + `ToolResult` sequence are
/// produced with their `tool_calls` field populated.
pub fn entries_to_messages(entries: &[ChatEntry]) -> Vec<LlmMessage> {
    let mut messages = Vec::new();

    for entry in entries {
        match &entry.kind {
            ChatEntryKind::User(text) => {
                messages.push(LlmMessage::User {
                    content: text.clone(),
                });
            }
            ChatEntryKind::Assistant(text) => {
                messages.push(LlmMessage::Assistant {
                    content: text.clone(),
                    tool_calls: None,
                });
            }
            ChatEntryKind::ToolCall {
                id,
                name,
                arguments,
            } => {
                // Attach tool calls to the most recent assistant message.
                // If there's no assistant message yet, create an empty one.
                let tool_call = ToolCall {
                    id: id.clone(),
                    name: name.clone(),
                    arguments: arguments.clone(),
                };
                match messages.last_mut() {
                    Some(LlmMessage::Assistant { tool_calls, .. }) => {
                        tool_calls.get_or_insert_with(Vec::new).push(tool_call);
                    }
                    _ => {
                        // Orphaned tool call — create an empty assistant message.
                        messages.push(LlmMessage::Assistant {
                            content: String::new(),
                            tool_calls: Some(vec![tool_call]),
                        });
                    }
                }
            }
            ChatEntryKind::ToolResult {
                id, name, content, ..
            } => {
                messages.push(LlmMessage::Tool {
                    tool_call_id: id.clone(),
                    name: name.clone(),
                    content: content.clone(),
                });
            }
            // System and Actor entries are not sent to the LLM.
            ChatEntryKind::System(_) | ChatEntryKind::Actor { .. } => {}
        }
    }

    messages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_to_messages_converts_user_entries() {
        // Given a user chat entry.
        let entries = vec![ChatEntry::user("hello")];

        // When converting to messages.
        let messages = entries_to_messages(&entries);

        // Then a single user message with correct content is produced.
        assert_eq!(messages.len(), 1);
        assert_eq!(
            messages[0],
            LlmMessage::User {
                content: "hello".into()
            }
        );
    }

    #[test]
    fn entries_to_messages_converts_assistant_entries() {
        // Given an assistant chat entry.
        let entries = vec![ChatEntry::assistant("hi there")];

        // When converting to messages.
        let messages = entries_to_messages(&entries);

        // Then a single assistant message with correct content is produced.
        assert_eq!(messages.len(), 1);
        assert_eq!(
            messages[0],
            LlmMessage::Assistant {
                content: "hi there".into(),
                tool_calls: None,
            }
        );
    }

    #[test]
    fn entries_to_messages_skips_system_and_actor() {
        // Given entries of all kinds.
        let entries = vec![
            ChatEntry::system("ready"),
            ChatEntry::user("hello"),
            ChatEntry::actor("echo", "HELLO"),
            ChatEntry::assistant("hi"),
        ];

        // When converting to messages.
        let messages = entries_to_messages(&entries);

        // Then only user and assistant messages are included.
        assert_eq!(messages.len(), 2);
        assert_eq!(
            messages[0],
            LlmMessage::User {
                content: "hello".into()
            }
        );
        assert_eq!(
            messages[1],
            LlmMessage::Assistant {
                content: "hi".into(),
                tool_calls: None,
            }
        );
    }

    #[test]
    fn entries_to_messages_empty_input() {
        // Given no entries.
        let entries: Vec<ChatEntry> = vec![];

        // When converting to messages.
        let messages = entries_to_messages(&entries);

        // Then no messages are produced.
        assert!(messages.is_empty());
    }

    #[test]
    fn entries_to_messages_converts_tool_call_entries() {
        // Given a tool call entry (orphaned — no preceding assistant).
        let entries = vec![ChatEntry::tool_call("call_1", "echo", r#"{"input":"hi"}"#)];

        // When converting to messages.
        let messages = entries_to_messages(&entries);

        // Then an empty assistant message with tool_calls is produced.
        assert_eq!(messages.len(), 1);
        match &messages[0] {
            LlmMessage::Assistant {
                content,
                tool_calls,
            } => {
                assert_eq!(content, "");
                let calls = tool_calls.as_ref().expect("should have tool_calls");
                assert_eq!(calls.len(), 1);
                assert_eq!(calls[0].id, "call_1");
                assert_eq!(calls[0].name, "echo");
            }
            other => panic!("expected Assistant, got {other:?}"),
        }
    }

    #[test]
    fn entries_to_messages_attaches_tool_calls_to_assistant() {
        // Given an assistant entry followed by a tool call entry.
        let entries = vec![
            ChatEntry::assistant("let me check"),
            ChatEntry::tool_call("call_1", "echo", r#"{"input":"hi"}"#),
        ];

        // When converting to messages.
        let messages = entries_to_messages(&entries);

        // Then one assistant message with tool_calls is produced.
        assert_eq!(messages.len(), 1);
        match &messages[0] {
            LlmMessage::Assistant {
                content,
                tool_calls,
            } => {
                assert_eq!(content, "let me check");
                let calls = tool_calls.as_ref().expect("should have tool_calls");
                assert_eq!(calls.len(), 1);
            }
            other => panic!("expected Assistant, got {other:?}"),
        }
    }

    #[test]
    fn entries_to_messages_converts_tool_result_entries() {
        // Given a tool result entry.
        let entries = vec![ChatEntry::tool_result("call_1", "echo", "hi", true)];

        // When converting to messages.
        let messages = entries_to_messages(&entries);

        // Then a Tool message is produced.
        assert_eq!(messages.len(), 1);
        assert_eq!(
            messages[0],
            LlmMessage::Tool {
                tool_call_id: "call_1".into(),
                name: "echo".into(),
                content: "hi".into(),
            }
        );
    }

    #[test]
    fn entries_to_messages_assembles_tool_loop() {
        // Given a full tool loop: user → assistant → tool call → tool result → assistant.
        let entries = vec![
            ChatEntry::user("what time is it?"),
            ChatEntry::assistant(""),
            ChatEntry::tool_call("call_1", "get_time", "{}"),
            ChatEntry::tool_result("call_1", "get_time", "12:00", true),
            ChatEntry::assistant("It's 12:00!"),
        ];

        // When converting to messages.
        let messages = entries_to_messages(&entries);

        // Then four messages are produced: user, assistant+tool_calls, tool, assistant.
        assert_eq!(messages.len(), 4);
        assert!(
            matches!(&messages[0], LlmMessage::User { content } if content == "what time is it?")
        );
        match &messages[1] {
            LlmMessage::Assistant {
                content,
                tool_calls,
            } => {
                assert_eq!(content, "");
                assert_eq!(tool_calls.as_ref().map(|v| v.len()), Some(1));
            }
            other => panic!("expected Assistant, got {other:?}"),
        }
        assert!(
            matches!(&messages[2], LlmMessage::Tool { tool_call_id, name, content }
            if tool_call_id == "call_1" && name == "get_time" && content == "12:00")
        );
        assert!(
            matches!(&messages[3], LlmMessage::Assistant { content, tool_calls }
            if content == "It's 12:00!" && tool_calls.is_none())
        );
    }

    #[test]
    fn entries_to_messages_multiple_tool_calls_in_one_response() {
        // Given an assistant entry followed by multiple tool call entries.
        let entries = vec![
            ChatEntry::assistant("checking both"),
            ChatEntry::tool_call("call_1", "echo", r#"{"input":"a"}"#),
            ChatEntry::tool_call("call_2", "get_time", "{}"),
        ];

        // When converting to messages.
        let messages = entries_to_messages(&entries);

        // Then one assistant message with two tool calls is produced.
        assert_eq!(messages.len(), 1);
        match &messages[0] {
            LlmMessage::Assistant {
                content,
                tool_calls,
            } => {
                assert_eq!(content, "checking both");
                let calls = tool_calls.as_ref().expect("should have tool_calls");
                assert_eq!(calls.len(), 2);
                assert_eq!(calls[0].id, "call_1");
                assert_eq!(calls[1].id, "call_2");
            }
            other => panic!("expected Assistant, got {other:?}"),
        }
    }

    #[test]
    fn entries_to_messages_skips_system_and_actor_with_tools() {
        // Given entries with system and actor entries between tool entries.
        let entries = vec![
            ChatEntry::user("go"),
            ChatEntry::assistant(""),
            ChatEntry::tool_call("call_1", "echo", "{}"),
            ChatEntry::system("some status"),
            ChatEntry::actor("actor-x", "doing work"),
            ChatEntry::tool_result("call_1", "echo", "ok", true),
        ];

        // When converting to messages.
        let messages = entries_to_messages(&entries);

        // Then system and actor entries are skipped.
        assert_eq!(messages.len(), 3);
        assert!(matches!(&messages[0], LlmMessage::User { .. }));
        assert!(matches!(&messages[1], LlmMessage::Assistant { .. }));
        assert!(matches!(&messages[2], LlmMessage::Tool { .. }));
    }
}
