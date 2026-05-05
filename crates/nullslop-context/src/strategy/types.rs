//! Core types for prompt assembly.

use async_trait::async_trait;
use error_stack::Report;
use nullslop_protocol::{ChatEntry, LlmMessage, PromptStrategyId, SessionId, ToolDefinition};
use wherror::Error;

/// Error type for prompt assembly operations.
#[derive(Debug, Error)]
#[error(debug)]
pub struct PromptAssemblyError;

/// The result of assembling a prompt for an LLM.
#[derive(Debug, Clone)]
pub struct AssembledPrompt {
    /// System prompt, if any. Will be prepended as `LlmMessage::System`.
    pub system_prompt: Option<String>,
    /// The assembled messages ready for the LLM.
    pub messages: Vec<LlmMessage>,
}

/// Context provided to a prompt assembly strategy.
///
/// Carries everything a strategy needs to produce an assembled prompt.
#[derive(Debug)]
pub struct AssemblyContext<'a> {
    /// The full conversation history for this session.
    pub history: &'a [ChatEntry],
    /// Tool definitions available for this session.
    pub tools: &'a [ToolDefinition],
    /// The name of the model being used.
    pub model_name: &'a str,
    /// The session this assembly is for.
    pub session_id: &'a SessionId,
}

/// Trait for prompt assembly strategies.
///
/// Each strategy receives raw history and produces the final LLM-ready output
/// (system prompt + messages). Strategies are black boxes — they own their own
/// internal logic and state.
#[async_trait]
pub trait PromptAssembly: Send + Sync {
    /// Assemble a prompt from the given context.
    ///
    /// # Errors
    ///
    /// Returns an error if assembly fails (e.g., token estimation overflow).
    async fn assemble(
        &self,
        context: &AssemblyContext<'_>,
    ) -> Result<AssembledPrompt, Report<PromptAssemblyError>>;

    /// The name of this strategy, for debugging.
    fn name(&self) -> &'static str;
}

/// Serializable state that a strategy can persist across sessions.
///
/// Strategies that maintain state (e.g., compaction summaries) implement this
/// trait. Stateless strategies (like passthrough) return `None` from `serialize`.
pub trait StrategySessionData: Send + Sync {
    /// Serialize the session data to an opaque JSON blob.
    fn serialize(&self) -> Option<serde_json::Value>;

    /// Deserialize session data from an opaque JSON blob.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails.
    fn deserialize(
        value: serde_json::Value,
    ) -> Result<Box<dyn StrategySessionData>, Report<PromptAssemblyError>>
    where
        Self: Sized;
}

/// Factory for creating prompt assembly strategies by ID.
pub trait StrategyFactory: Send + Sync {
    /// Create a strategy instance for the given ID.
    ///
    /// Returns `None` if the ID is not recognized.
    ///
    /// # Errors
    ///
    /// Returns an error if strategy creation fails.
    fn create(
        &self,
        id: &PromptStrategyId,
    ) -> Result<Box<dyn PromptAssembly>, Report<PromptAssemblyError>>;

    /// The name of this factory, for debugging.
    fn name(&self) -> &'static str;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assembled_prompt_with_system_message() {
        // Given an assembled prompt with a system prompt and messages.
        let prompt = AssembledPrompt {
            system_prompt: Some("You are helpful.".to_owned()),
            messages: vec![LlmMessage::User {
                content: "hello".to_owned(),
            }],
        };

        // Then the fields are correct.
        assert_eq!(prompt.system_prompt.as_deref(), Some("You are helpful."));
        assert_eq!(prompt.messages.len(), 1);
    }

    #[test]
    fn assembled_prompt_without_system_message() {
        // Given an assembled prompt without a system prompt.
        let prompt = AssembledPrompt {
            system_prompt: None,
            messages: vec![],
        };

        // Then system_prompt is None.
        assert!(prompt.system_prompt.is_none());
        assert!(prompt.messages.is_empty());
    }
}
