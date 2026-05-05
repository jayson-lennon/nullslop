//! Compaction strategy — stub that detects when context exceeds a threshold.
//!
//! When the estimated token count of the full history is within the budget,
//! this strategy behaves like passthrough (all entries, no system prompt).
//! When it exceeds the budget, it trims newest-to-oldest (like
//! [`TokenBudgetStrategy`](super::token_budget::TokenBudgetStrategy)) and
//! sets a compaction-specific system prompt.
//!
//! The full implementation (LLM-based summarization, summary storage,
//! incremental compaction) is a follow-up task. This stub validates the
//! architecture and persistence plumbing.

use async_trait::async_trait;
use error_stack::Report;
use nullslop_protocol::{ChatEntry, entries_to_messages};

use crate::strategy::token_estimator::{TokenEstimator, estimate_entry_tokens};
use crate::strategy::types::{
    AssembledPrompt, AssemblyContext, PromptAssembly, PromptAssemblyError,
};

/// System prompt set when context was compacted (stub: trimmed).
const COMPACTION_SYSTEM_PROMPT: &str = "Context was compacted to fit within the token budget. Earlier conversation history was summarized.";

/// A compaction strategy stub that trims context when it exceeds a token threshold.
///
/// In the full implementation, this will use LLM-based summarization instead
/// of simple trimming. For now, it falls back to token-budget-style trimming
/// with a compaction-specific system prompt.
pub struct CompactionStrategy {
    /// Maximum estimated tokens before compaction triggers.
    max_tokens: usize,
    /// Token estimator for budgeting.
    estimator: Box<dyn TokenEstimator>,
}

impl CompactionStrategy {
    /// Create a new compaction strategy with the given threshold and estimator.
    #[must_use]
    pub fn new(max_tokens: usize, estimator: Box<dyn TokenEstimator>) -> Self {
        Self {
            max_tokens,
            estimator,
        }
    }
}

#[async_trait]
impl PromptAssembly for CompactionStrategy {
    async fn assemble(
        &self,
        context: &AssemblyContext<'_>,
    ) -> Result<AssembledPrompt, Report<PromptAssemblyError>> {
        if context.history.is_empty() {
            return Ok(AssembledPrompt {
                system_prompt: None,
                messages: vec![],
            });
        }

        // Estimate total tokens across all history.
        let total_tokens: usize = context
            .history
            .iter()
            .map(|entry| estimate_entry_tokens(self.estimator.as_ref(), entry))
            .sum();

        // If everything fits, delegate to passthrough behavior.
        if total_tokens <= self.max_tokens {
            let messages = entries_to_messages(context.history);
            return Ok(AssembledPrompt {
                system_prompt: None,
                messages,
            });
        }

        // Over threshold — trim newest-to-oldest (stub behavior).
        let mut included_indices = Vec::new();
        let mut used_tokens = 0usize;

        for (i, entry) in context.history.iter().enumerate().rev() {
            let entry_tokens = estimate_entry_tokens(self.estimator.as_ref(), entry);

            // Always include at least the most recent entry.
            if !included_indices.is_empty() && used_tokens + entry_tokens > self.max_tokens {
                break;
            }

            used_tokens += entry_tokens;
            included_indices.push(i);
        }

        // Sort indices back to chronological order.
        included_indices.sort_unstable();

        let included: Vec<&ChatEntry> = included_indices
            .iter()
            .map(|&i| {
                // SAFETY: indices come from enumerate on context.history
                unsafe { context.history.get_unchecked(i) }
            })
            .collect();

        let messages = entries_to_messages(&included.into_iter().cloned().collect::<Vec<_>>());

        Ok(AssembledPrompt {
            system_prompt: Some(COMPACTION_SYSTEM_PROMPT.to_owned()),
            messages,
        })
    }

    fn name(&self) -> &'static str {
        "compaction"
    }
}

#[cfg(test)]
mod tests {
    use nullslop_protocol::{ChatEntry, SessionId};

    use super::*;
    use crate::strategy::token_estimator::CharRatioEstimator;

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

    fn make_strategy(max_tokens: usize) -> CompactionStrategy {
        CompactionStrategy::new(max_tokens, Box::new(CharRatioEstimator))
    }

    #[tokio::test]
    async fn returns_all_entries_when_under_threshold() {
        // Given entries that fit within the threshold.
        let history = vec![
            ChatEntry::user("hi"),
            ChatEntry::assistant("hello"),
            ChatEntry::user("how are you?"),
        ];
        let strategy = make_strategy(8192);
        let session_id = SessionId::new();
        let context = test_context(&history, &session_id);

        // When assembling.
        let result = strategy.assemble(&context).await.expect("assemble");

        // Then all entries are included with no system prompt.
        assert_eq!(result.messages.len(), 3);
        assert!(result.system_prompt.is_none());
    }

    #[tokio::test]
    async fn trims_entries_when_over_threshold() {
        // Given entries that exceed the threshold.
        let history: Vec<ChatEntry> = std::iter::repeat_with(|| ChatEntry::user("a".repeat(400)))
            .take(10)
            .collect();
        let strategy = make_strategy(100);
        let session_id = SessionId::new();
        let context = test_context(&history, &session_id);

        // When assembling.
        let result = strategy.assemble(&context).await.expect("assemble");

        // Then fewer entries are included with a compaction system prompt.
        assert!(result.messages.len() < 10);
        assert!(result.system_prompt.is_some());
        assert_eq!(
            result.system_prompt.as_deref(),
            Some(
                "Context was compacted to fit within the token budget. Earlier conversation history was summarized."
            )
        );
    }

    #[tokio::test]
    async fn empty_history_produces_no_messages() {
        // Given empty history.
        let history: Vec<ChatEntry> = vec![];
        let strategy = make_strategy(8192);
        let session_id = SessionId::new();
        let context = test_context(&history, &session_id);

        // When assembling.
        let result = strategy.assemble(&context).await.expect("assemble");

        // Then no messages are produced.
        assert!(result.messages.is_empty());
        assert!(result.system_prompt.is_none());
    }

    #[tokio::test]
    async fn single_over_threshold_entry_is_included_anyway() {
        // Given one entry that far exceeds the threshold.
        let history = vec![ChatEntry::user("x".repeat(1000))];
        let strategy = make_strategy(10);
        let session_id = SessionId::new();
        let context = test_context(&history, &session_id);

        // When assembling.
        let result = strategy.assemble(&context).await.expect("assemble");

        // Then the entry is still included (over threshold triggers trimming).
        assert_eq!(result.messages.len(), 1);
        // System prompt is set because we're over threshold.
        assert!(result.system_prompt.is_some());
    }

    #[tokio::test]
    async fn compaction_system_prompt_differs_from_token_budget() {
        // Given entries that trigger compaction.
        let history = vec![
            ChatEntry::user("a".repeat(200)),
            ChatEntry::assistant("b".repeat(200)),
            ChatEntry::user("short"),
        ];
        let strategy = make_strategy(30);
        let session_id = SessionId::new();
        let context = test_context(&history, &session_id);

        // When assembling.
        let result = strategy.assemble(&context).await.expect("assemble");

        // Then the compaction system prompt is distinct from token budget's.
        assert_ne!(
            result.system_prompt.as_deref(),
            Some("Some earlier context was omitted to fit within the token budget.")
        );
        assert_eq!(
            result.system_prompt.as_deref(),
            Some(
                "Context was compacted to fit within the token budget. Earlier conversation history was summarized."
            )
        );
    }

    #[tokio::test]
    async fn preserves_chronological_order() {
        // Given 3 entries where trimming occurs.
        let history = vec![
            ChatEntry::user("a".repeat(200)),
            ChatEntry::assistant("b".repeat(200)),
            ChatEntry::user("short"),
        ];
        let strategy = make_strategy(60);
        let session_id = SessionId::new();
        let context = test_context(&history, &session_id);

        // When assembling.
        let result = strategy.assemble(&context).await.expect("assemble");

        // Then the included messages maintain chronological order.
        assert!(!result.messages.is_empty());
        let last = result.messages.last().expect("should have messages");
        assert_eq!(
            last,
            &nullslop_protocol::LlmMessage::User {
                content: "short".to_owned(),
            }
        );
    }

    #[tokio::test]
    async fn name_returns_compaction() {
        // Given a compaction strategy.
        let strategy = make_strategy(8192);

        // Then its name is "compaction".
        assert_eq!(strategy.name(), "compaction");
    }
}
