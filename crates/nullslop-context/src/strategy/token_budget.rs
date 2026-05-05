//! Token budget strategy — limits context to fit within a token budget.
//!
//! Walks history from newest to oldest, accumulating token estimates.
//! When the budget is exceeded, older entries are dropped. Always includes
//! at least the most recent entry so the user's message is never lost.
//! Sets a system prompt when context was trimmed to inform the LLM.

use async_trait::async_trait;
use error_stack::Report;
use nullslop_protocol::{ChatEntry, entries_to_messages};

use crate::strategy::token_estimator::{TokenEstimator, estimate_entry_tokens};
use crate::strategy::types::{
    AssembledPrompt, AssemblyContext, PromptAssembly, PromptAssemblyError,
};

/// System prompt set when context was trimmed to fit the budget.
const TRIMMED_SYSTEM_PROMPT: &str =
    "Some earlier context was omitted to fit within the token budget.";

/// A strategy that limits context to fit within a configurable token budget.
///
/// Walks history from newest to oldest, accumulating token estimates.
/// Stops when adding another entry would exceed the budget. Always includes
/// at least the most recent entry even if it exceeds the budget.
pub struct TokenBudgetStrategy {
    /// Maximum number of tokens to include.
    max_tokens: usize,
    /// Token estimator for computing token counts.
    estimator: Box<dyn TokenEstimator>,
}

impl TokenBudgetStrategy {
    /// Create a new token budget strategy with the given budget and estimator.
    #[must_use]
    pub fn new(max_tokens: usize, estimator: Box<dyn TokenEstimator>) -> Self {
        Self {
            max_tokens,
            estimator,
        }
    }
}

#[async_trait]
impl PromptAssembly for TokenBudgetStrategy {
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

        // Walk history from newest to oldest, accumulating token estimates.
        let mut included_indices = Vec::new();
        let mut total_tokens = 0usize;

        for (i, entry) in context.history.iter().enumerate().rev() {
            let entry_tokens = estimate_entry_tokens(self.estimator.as_ref(), entry);

            // Always include at least the most recent entry.
            if !included_indices.is_empty() && total_tokens + entry_tokens > self.max_tokens {
                break;
            }

            total_tokens += entry_tokens;
            included_indices.push(i);
        }

        // Sort indices back to chronological order (oldest to newest).
        included_indices.sort();

        // Collect included entries.
        let included: Vec<&ChatEntry> = included_indices
            .iter()
            .map(|&i| &context.history[i])
            .collect();

        let trimmed = included.len() < context.history.len();
        let messages = entries_to_messages(&included.into_iter().cloned().collect::<Vec<_>>());

        Ok(AssembledPrompt {
            system_prompt: if trimmed {
                Some(TRIMMED_SYSTEM_PROMPT.to_owned())
            } else {
                None
            },
            messages,
        })
    }

    fn name(&self) -> &'static str {
        "token_budget"
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

    fn make_strategy(max_tokens: usize) -> TokenBudgetStrategy {
        TokenBudgetStrategy::new(max_tokens, Box::new(CharRatioEstimator))
    }

    #[tokio::test]
    async fn truncates_history_to_fit_budget() {
        // Given 5 entries with ~100-char content each (~26 tokens each, ~130 total)
        // and a budget of 80 tokens.
        let history: Vec<ChatEntry> = (0..5)
            .map(|i| ChatEntry::user("a".repeat(100) + &i.to_string()))
            .collect();
        let strategy = make_strategy(80);
        let session_id = SessionId::new();
        let context = test_context(&history, &session_id);

        // When assembling.
        let result = strategy.assemble(&context).await.expect("assemble");

        // Then fewer than 5 entries are included and system prompt is set.
        assert!(result.messages.len() < 5);
        assert!(result.system_prompt.is_some());
    }

    #[tokio::test]
    async fn returns_all_entries_when_under_budget() {
        // Given 3 short entries that easily fit in a large budget.
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
    async fn single_over_budget_entry_is_included_anyway() {
        // Given one entry that far exceeds the budget.
        let history = vec![ChatEntry::user(&"x".repeat(1000))];
        let strategy = make_strategy(10);
        let session_id = SessionId::new();
        let context = test_context(&history, &session_id);

        // When assembling.
        let result = strategy.assemble(&context).await.expect("assemble");

        // Then the entry is still included (no trimming occurred on a single entry).
        assert_eq!(result.messages.len(), 1);
        assert!(result.system_prompt.is_none());
    }

    #[tokio::test]
    async fn system_prompt_set_when_trimmed() {
        // Given entries that exceed the budget.
        let history = vec![
            ChatEntry::user(&"a".repeat(200)),
            ChatEntry::assistant(&"b".repeat(200)),
            ChatEntry::user("short"),
        ];
        let strategy = make_strategy(30);
        let session_id = SessionId::new();
        let context = test_context(&history, &session_id);

        // When assembling.
        let result = strategy.assemble(&context).await.expect("assemble");

        // Then system prompt is set indicating context was trimmed.
        assert!(result.system_prompt.is_some());
        assert_eq!(
            result.system_prompt.as_deref(),
            Some("Some earlier context was omitted to fit within the token budget.")
        );
    }

    #[tokio::test]
    async fn no_system_prompt_when_nothing_trimmed() {
        // Given entries that fit within the budget.
        let history = vec![ChatEntry::user("hi"), ChatEntry::assistant("hello")];
        let strategy = make_strategy(8192);
        let session_id = SessionId::new();
        let context = test_context(&history, &session_id);

        // When assembling.
        let result = strategy.assemble(&context).await.expect("assemble");

        // Then no system prompt is set.
        assert!(result.system_prompt.is_none());
    }

    #[tokio::test]
    async fn preserves_chronological_order() {
        // Given 3 entries where the first exceeds the budget when combined.
        let history = vec![
            ChatEntry::user(&"a".repeat(200)),
            ChatEntry::assistant(&"b".repeat(200)),
            ChatEntry::user("short"),
        ];
        let strategy = make_strategy(60);
        let session_id = SessionId::new();
        let context = test_context(&history, &session_id);

        // When assembling.
        let result = strategy.assemble(&context).await.expect("assemble");

        // Then the included messages maintain chronological order.
        assert!(result.messages.len() >= 1);
        // The last message should be the most recent ("short" user message).
        let last = result.messages.last().expect("should have messages");
        assert_eq!(
            last,
            &nullslop_protocol::LlmMessage::User {
                content: "short".to_owned(),
            }
        );
    }

    #[tokio::test]
    async fn name_returns_token_budget() {
        // Given a token budget strategy.
        let strategy = make_strategy(8192);

        // Then its name is "token_budget".
        assert_eq!(strategy.name(), "token_budget");
    }

    #[tokio::test]
    async fn skips_system_and_actor_entries() {
        // Given entries including system and actor types.
        let history = vec![
            ChatEntry::system("status"),
            ChatEntry::user("hi"),
            ChatEntry::actor("echo", "HELLO"),
            ChatEntry::assistant("hello"),
        ];
        let strategy = make_strategy(8192);
        let session_id = SessionId::new();
        let context = test_context(&history, &session_id);

        // When assembling.
        let result = strategy.assemble(&context).await.expect("assemble");

        // Then system and actor entries are skipped (they contribute 0 tokens).
        assert_eq!(result.messages.len(), 2);
        assert!(result.system_prompt.is_none());
    }

    #[tokio::test]
    async fn newest_entry_included_when_rest_trimmed() {
        // Given many entries where only the newest fits.
        let mut history = Vec::new();
        for _ in 0..10 {
            history.push(ChatEntry::user(&"x".repeat(100)));
        }
        // Most recent is short.
        history.push(ChatEntry::user("ok"));
        let strategy = make_strategy(10);
        let session_id = SessionId::new();
        let context = test_context(&history, &session_id);

        // When assembling.
        let result = strategy.assemble(&context).await.expect("assemble");

        // Then at least the most recent entry is included.
        assert!(!result.messages.is_empty());
        let last = result.messages.last().expect("should have messages");
        assert_eq!(
            last,
            &nullslop_protocol::LlmMessage::User {
                content: "ok".to_owned(),
            }
        );
    }
}
