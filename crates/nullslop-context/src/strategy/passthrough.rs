//! Passthrough strategy — sends all entries with no filtering or system prompt.
//!
//! This is the simplest strategy and produces identical behavior to the
//! original direct conversion. It uses [`entries_to_messages`] internally.

use async_trait::async_trait;
use error_stack::Report;
use nullslop_protocol::entries_to_messages;

use crate::strategy::types::{
    AssembledPrompt, AssemblyContext, PromptAssembly, PromptAssemblyError,
};

/// A passthrough strategy that sends all entries unchanged.
///
/// No system prompt, no filtering. Equivalent to the original `entries_to_messages`
/// conversion that was done inline in the message queue handler.
pub struct PassthroughStrategy;

#[async_trait]
impl PromptAssembly for PassthroughStrategy {
    async fn assemble(
        &self,
        context: &AssemblyContext<'_>,
    ) -> Result<AssembledPrompt, Report<PromptAssemblyError>> {
        let messages = entries_to_messages(context.history);
        Ok(AssembledPrompt {
            system_prompt: None,
            messages,
        })
    }

    fn name(&self) -> &'static str {
        "passthrough"
    }
}

#[cfg(test)]
mod tests {
    use nullslop_protocol::{ChatEntry, SessionId};

    use super::*;

    fn test_context<'a>(
        history: &'a [ChatEntry],
        session_id: &'a SessionId,
    ) -> AssemblyContext<'a> {
        AssemblyContext {
            history,
            tools: &[],
            model_name: "test-model",
            session_id,
        }
    }

    #[tokio::test]
    async fn passthrough_converts_all_entries() {
        // Given a history with user and assistant entries.
        let history = vec![
            ChatEntry::user("hello"),
            ChatEntry::assistant("hi there"),
            ChatEntry::user("how are you?"),
        ];

        // When assembling with passthrough strategy.
        let strategy = PassthroughStrategy;
        let session_id = SessionId::new();
        let context = test_context(&history, &session_id);
        let result = strategy.assemble(&context).await.expect("assemble");

        // Then all entries are converted and there is no system prompt.
        assert!(result.system_prompt.is_none());
        assert_eq!(result.messages.len(), 3);
    }

    #[tokio::test]
    async fn passthrough_skips_system_and_actor_entries() {
        // Given a history with mixed entry types.
        let history = vec![
            ChatEntry::system("ready"),
            ChatEntry::user("hello"),
            ChatEntry::actor("echo", "HELLO"),
            ChatEntry::assistant("hi"),
        ];

        // When assembling with passthrough strategy.
        let strategy = PassthroughStrategy;
        let session_id = SessionId::new();
        let context = test_context(&history, &session_id);
        let result = strategy.assemble(&context).await.expect("assemble");

        // Then system and actor entries are skipped (matching entries_to_messages).
        assert_eq!(result.messages.len(), 2);
    }

    #[tokio::test]
    async fn passthrough_empty_history() {
        // Given empty history.
        let history: Vec<ChatEntry> = vec![];

        // When assembling with passthrough strategy.
        let strategy = PassthroughStrategy;
        let session_id = SessionId::new();
        let context = test_context(&history, &session_id);
        let result = strategy.assemble(&context).await.expect("assemble");

        // Then no messages are produced.
        assert!(result.messages.is_empty());
        assert!(result.system_prompt.is_none());
    }

    #[tokio::test]
    async fn passthrough_name() {
        // Given a passthrough strategy.
        let strategy = PassthroughStrategy;

        // Then its name is "passthrough".
        assert_eq!(strategy.name(), "passthrough");
    }

    #[tokio::test]
    async fn passthrough_preserves_tool_calls() {
        // Given a history with a tool loop.
        let history = vec![
            ChatEntry::user("go"),
            ChatEntry::assistant("checking"),
            ChatEntry::tool_call("call_1", "echo", r#"{"input":"hi"}"#),
            ChatEntry::tool_result("call_1", "echo", "hi", true),
            ChatEntry::assistant("done!"),
        ];

        // When assembling with passthrough strategy.
        let strategy = PassthroughStrategy;
        let session_id = SessionId::new();
        let context = test_context(&history, &session_id);
        let result = strategy.assemble(&context).await.expect("assemble");

        // Then tool calls are preserved correctly.
        assert_eq!(result.messages.len(), 4);
    }
}
