//! Conversion from protocol LLM messages to llm crate messages.

use llm::chat::ChatMessage;
use nullslop_protocol::provider::{LlmMessage, LlmRole};

/// Convert a single protocol [`LlmMessage`] to an llm crate [`ChatMessage`].
fn convert_message(msg: &LlmMessage) -> ChatMessage {
    match msg.role {
        LlmRole::User => ChatMessage::user().content(&msg.content).build(),
        LlmRole::Assistant => ChatMessage::assistant().content(&msg.content).build(),
    }
}

/// Convert protocol LLM messages to llm crate messages.
///
/// This is a thin wrapper that maps each [`LlmMessage`] to a
/// [`ChatMessage`] using the role-to-builder conversion.
pub fn llm_messages_to_chat_messages(messages: &[LlmMessage]) -> Vec<ChatMessage> {
    messages.iter().map(convert_message).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_user_message_to_chat_message() {
        // Given a protocol user message.
        let msg = LlmMessage {
            role: LlmRole::User,
            content: "hello".to_owned(),
        };

        // When converting to ChatMessage.
        let chat_msg = convert_message(&msg);

        // Then content and role are correct.
        assert_eq!(chat_msg.content, "hello");
        assert!(matches!(chat_msg.role, llm::chat::ChatRole::User));
    }

    #[test]
    fn convert_assistant_message_to_chat_message() {
        // Given a protocol assistant message.
        let msg = LlmMessage {
            role: LlmRole::Assistant,
            content: "hi there".to_owned(),
        };

        // When converting to ChatMessage.
        let chat_msg = convert_message(&msg);

        // Then content and role are correct.
        assert_eq!(chat_msg.content, "hi there");
        assert!(matches!(chat_msg.role, llm::chat::ChatRole::Assistant));
    }

    #[test]
    fn llm_messages_to_chat_messages_converts_list() {
        // Given a list of protocol messages.
        let messages = vec![
            LlmMessage {
                role: LlmRole::User,
                content: "hello".to_owned(),
            },
            LlmMessage {
                role: LlmRole::Assistant,
                content: "hi".to_owned(),
            },
        ];

        // When converting.
        let result = llm_messages_to_chat_messages(&messages);

        // Then both are converted correctly.
        assert_eq!(result.len(), 2);
        assert!(matches!(result[0].role, llm::chat::ChatRole::User));
        assert!(matches!(result[1].role, llm::chat::ChatRole::Assistant));
    }
}
