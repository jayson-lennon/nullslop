//! Sliding window strategy — sends only the last N entries from history.
//!
//! This strategy limits context to a configurable window of the most recent
//! entries, dropping older ones. It uses [`entries_to_messages`] internally
//! and produces no system prompt.

use async_trait::async_trait;
use error_stack::Report;
use nullslop_protocol::entries_to_messages;

use crate::strategy::types::{
    AssembledPrompt, AssemblyContext, PromptAssembly, PromptAssemblyError,
};

/// A sliding window strategy that sends only the last `window_size` entries.
pub struct SlidingWindowStrategy {
    /// Maximum number of history entries to include.
    window_size: usize,
}

impl SlidingWindowStrategy {
    /// Create a new sliding window strategy with the given window size.
    #[must_use]
    pub fn new(window_size: usize) -> Self {
        Self { window_size }
    }
}

#[async_trait]
impl PromptAssembly for SlidingWindowStrategy {
    async fn assemble(
        &self,
        context: &AssemblyContext<'_>,
    ) -> Result<AssembledPrompt, Report<PromptAssemblyError>> {
        let window = if context.history.len() > self.window_size {
            &context.history[context.history.len() - self.window_size..]
        } else {
            context.history
        };
        let messages = entries_to_messages(window);
        Ok(AssembledPrompt {
            system_prompt: None,
            messages,
        })
    }

    fn name(&self) -> &'static str {
        "sliding_window"
    }
}

#[cfg(test)]
mod tests {
    use nullslop_protocol::{ChatEntry, SessionId};
    use super::*;

    fn test_context<'a>(history: &'a [ChatEntry], session_id: &'a SessionId) -> AssemblyContext<'a> {
        AssemblyContext {
            history,
            tools: &[],
            model_name: "test-model",
            session_id,
        }
    }

    #[tokio::test]
    async fn sliding_window_truncates_history() {
        // Given 5 entries and a window of 3.
        let history = vec![
            ChatEntry::user("msg1"),
            ChatEntry::assistant("reply1"),
            ChatEntry::user("msg2"),
            ChatEntry::assistant("reply2"),
            ChatEntry::user("msg3"),
        ];
        let strategy = SlidingWindowStrategy::new(3);
        let session_id = SessionId::new();
        let context = test_context(&history, &session_id);

        // When assembling.
        let result = strategy.assemble(&context).await.expect("assemble");

        // Then only the last 3 entries are included (2 user messages, 1 assistant).
        assert!(result.system_prompt.is_none());
        assert_eq!(result.messages.len(), 3);
        assert_eq!(
            result.messages[0],
            nullslop_protocol::LlmMessage::User {
                content: "msg2".to_owned(),
            }
        );
    }

    #[tokio::test]
    async fn sliding_window_returns_all_when_under_limit() {
        // Given 2 entries and a window of 5.
        let history = vec![
            ChatEntry::user("hello"),
            ChatEntry::assistant("hi"),
        ];
        let strategy = SlidingWindowStrategy::new(5);
        let session_id = SessionId::new();
        let context = test_context(&history, &session_id);

        // When assembling.
        let result = strategy.assemble(&context).await.expect("assemble");

        // Then all entries are included.
        assert_eq!(result.messages.len(), 2);
    }

    #[tokio::test]
    async fn sliding_window_empty_history() {
        // Given no entries.
        let history: Vec<ChatEntry> = vec![];
        let strategy = SlidingWindowStrategy::new(10);
        let session_id = SessionId::new();
        let context = test_context(&history, &session_id);

        // When assembling.
        let result = strategy.assemble(&context).await.expect("assemble");

        // Then no messages are produced.
        assert!(result.messages.is_empty());
        assert!(result.system_prompt.is_none());
    }

    #[tokio::test]
    async fn sliding_window_exact_size() {
        // Given 3 entries and a window of 3.
        let history = vec![
            ChatEntry::user("a"),
            ChatEntry::user("b"),
            ChatEntry::user("c"),
        ];
        let strategy = SlidingWindowStrategy::new(3);
        let session_id = SessionId::new();
        let context = test_context(&history, &session_id);

        // When assembling.
        let result = strategy.assemble(&context).await.expect("assemble");

        // Then all 3 entries are included.
        assert_eq!(result.messages.len(), 3);
    }

    #[tokio::test]
    async fn sliding_window_name() {
        let strategy = SlidingWindowStrategy::new(10);
        assert_eq!(strategy.name(), "sliding_window");
    }

    #[tokio::test]
    async fn sliding_window_skips_non_llm_entries_in_window() {
        // Given 4 entries where one is a system entry.
        let history = vec![
            ChatEntry::user("msg1"),
            ChatEntry::system("status update"),
            ChatEntry::assistant("reply"),
            ChatEntry::user("msg2"),
        ];
        let strategy = SlidingWindowStrategy::new(4);
        let session_id = SessionId::new();
        let context = test_context(&history, &session_id);

        // When assembling.
        let result = strategy.assemble(&context).await.expect("assemble");

        // Then the system entry is skipped by entries_to_messages.
        assert_eq!(result.messages.len(), 3);
    }
}
