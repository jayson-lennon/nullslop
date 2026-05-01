//! Conversion from chat entries to LLM messages.

use super::message::{LlmMessage, LlmRole};
use crate::ChatEntry;
use crate::ChatEntryKind;

/// Convert chat history entries to LLM messages.
///
/// Only `User` and `Assistant` entries are included. System and actor entries
/// are skipped since they are not part of the conversation context for the LLM.
pub fn entries_to_messages(entries: &[ChatEntry]) -> Vec<LlmMessage> {
    entries
        .iter()
        .filter_map(|entry| match &entry.kind {
            ChatEntryKind::User(text) => Some(LlmMessage {
                role: LlmRole::User,
                content: text.clone(),
            }),
            ChatEntryKind::Assistant(text) => Some(LlmMessage {
                role: LlmRole::Assistant,
                content: text.clone(),
            }),
            _ => None,
        })
        .collect()
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
        assert_eq!(messages[0].role, LlmRole::User);
        assert_eq!(messages[0].content, "hello");
    }

    #[test]
    fn entries_to_messages_converts_assistant_entries() {
        // Given an assistant chat entry.
        let entries = vec![ChatEntry::assistant("hi there")];

        // When converting to messages.
        let messages = entries_to_messages(&entries);

        // Then a single assistant message with correct content is produced.
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, LlmRole::Assistant);
        assert_eq!(messages[0].content, "hi there");
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
        assert_eq!(messages[0].role, LlmRole::User);
        assert_eq!(messages[1].role, LlmRole::Assistant);
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
}
