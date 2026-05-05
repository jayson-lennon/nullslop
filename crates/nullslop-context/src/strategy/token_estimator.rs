//! Token estimation for prompt assembly budgeting.
//!
//! Provides a [`TokenEstimator`] trait for estimating token counts from text,
//! [`CharRatioEstimator`] as a simple heuristic implementation (1 token ≈ 4 characters),
//! and [`estimate_entry_tokens`] for estimating the token cost of individual chat entries.

use nullslop_protocol::{ChatEntry, ChatEntryKind};

/// Estimates the token count of text.
///
/// Used by budget-based strategies to decide how much history to include.
/// Implementations may range from simple heuristics to real tokenizer calls.
pub trait TokenEstimator: Send + Sync {
    /// Estimate the number of tokens in `text`.
    fn estimate(&self, text: &str) -> usize;

    /// The name of this estimator, for debugging.
    fn name(&self) -> &'static str;
}

/// Estimate the token count for a single chat entry's text content.
///
/// Uses the same fields that `entries_to_messages` would convert to LLM messages:
/// [`ChatEntryKind::User`]/[`ChatEntryKind::Assistant`] content, [`ChatEntryKind::ToolCall`] name+arguments, [`ChatEntryKind::ToolResult`] name+content.
/// System and Actor entries contribute 0 tokens since they are not sent to the LLM.
pub fn estimate_entry_tokens(estimator: &dyn TokenEstimator, entry: &ChatEntry) -> usize {
    match &entry.kind {
        ChatEntryKind::User(text) | ChatEntryKind::Assistant(text) => estimator.estimate(text),
        ChatEntryKind::ToolCall {
            name, arguments, ..
        } => estimator.estimate(name) + estimator.estimate(arguments),
        ChatEntryKind::ToolResult { name, content, .. } => {
            estimator.estimate(name) + estimator.estimate(content)
        }
        // System and Actor entries are not sent to the LLM.
        ChatEntryKind::System(_) | ChatEntryKind::Actor { .. } => 0,
    }
}

/// Simple heuristic estimator: 1 token ≈ 4 Unicode characters.
///
/// Good enough for initial use. Uses `text.chars().count()` for Unicode correctness
/// rather than byte length. Always returns at least 1 to avoid zero-token estimates.
pub struct CharRatioEstimator;

impl TokenEstimator for CharRatioEstimator {
    #[expect(clippy::integer_division, reason = "1 token ≈ 4 characters is intentional rounding")]
    fn estimate(&self, text: &str) -> usize {
        text.chars().count() / 4 + 1
    }

    fn name(&self) -> &'static str {
        "char_ratio"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn char_ratio_returns_nonzero_for_empty_string() {
        // Given a char ratio estimator.
        let estimator = CharRatioEstimator;

        // When estimating an empty string.
        let tokens = estimator.estimate("");

        // Then at least 1 token is returned.
        assert!(tokens >= 1);
    }

    #[test]
    fn char_ratio_estimates_reasonably() {
        // Given a char ratio estimator and a 100-character string.
        let estimator = CharRatioEstimator;
        let text = "a".repeat(100);

        // When estimating tokens.
        let tokens = estimator.estimate(&text);

        // Then approximately 25 tokens are returned (100/4 + 1 = 26).
        assert_eq!(tokens, 26);
    }

    #[test]
    fn char_ratio_name() {
        // Given a char ratio estimator.
        let estimator = CharRatioEstimator;

        // Then its name is "char_ratio".
        assert_eq!(estimator.name(), "char_ratio");
    }

    #[test]
    fn char_ratio_handles_unicode_correctly() {
        // Given a char ratio estimator and a string with multi-byte characters.
        let estimator = CharRatioEstimator;
        // "日本語" is 3 Unicode characters but 9 bytes in UTF-8.
        let text = "日本語";

        // When estimating tokens.
        let tokens = estimator.estimate(text);

        // Then it uses character count (3), not byte count (3 * 3/4 = 2, rounded = 1).
        assert_eq!(tokens, 1);
    }

    #[test]
    fn estimate_entry_tokens_for_user() {
        // Given a char ratio estimator and a user entry.
        let estimator = CharRatioEstimator;
        let entry = nullslop_protocol::ChatEntry::user("hello world");

        // When estimating entry tokens.
        let tokens = estimate_entry_tokens(&estimator, &entry);

        // Then it matches estimating the user text directly.
        assert_eq!(tokens, estimator.estimate("hello world"));
    }

    #[test]
    fn estimate_entry_tokens_for_tool_call() {
        // Given a char ratio estimator and a tool call entry.
        let estimator = CharRatioEstimator;
        let entry = nullslop_protocol::ChatEntry::tool_call("call_1", "echo", r#"{"input":"hi"}"#);

        // When estimating entry tokens.
        let tokens = estimate_entry_tokens(&estimator, &entry);

        // Then it estimates name + arguments combined.
        assert_eq!(
            tokens,
            estimator.estimate("echo") + estimator.estimate(r#"{"input":"hi"}"#)
        );
    }

    #[test]
    fn estimate_entry_tokens_for_system_is_zero() {
        // Given a char ratio estimator and a system entry.
        let estimator = CharRatioEstimator;
        let entry = nullslop_protocol::ChatEntry::system("some status message");

        // When estimating entry tokens.
        let tokens = estimate_entry_tokens(&estimator, &entry);

        // Then system entries contribute 0 tokens.
        assert_eq!(tokens, 0);
    }
}
