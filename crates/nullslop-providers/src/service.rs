//! LLM service trait and error types.

use error_stack::Report;
use futures::stream::Stream;
use llm::chat::ChatMessage;
use std::pin::Pin;
use wherror::Error;

/// Error type for LLM service operations.
///
/// Unit variants — the original error is preserved in the `Report` chain
/// via `.change_context()`. Attach additional context with `.attach()`.
#[derive(Debug, Error)]
pub enum LlmServiceError {
    /// API key not found or invalid.
    #[error("API key error")]
    ApiKey,
    /// The LLM provider returned an error.
    #[error("LLM provider error")]
    Provider,
    /// Builder configuration error.
    #[error("LLM configuration error")]
    Config,
}

/// A streaming LLM chat response.
pub type ChatStream = Pin<Box<dyn Stream<Item = Result<String, Report<LlmServiceError>>> + Send>>;

/// Trait for a single LLM streaming chat session.
///
/// Use [`LlmServiceFactory`] to create instances.
#[async_trait::async_trait]
pub trait LlmService: Send + Sync {
    /// Start a streaming chat completion.
    ///
    /// Returns a stream of text tokens. The stream ends when the LLM finishes
    /// generating or errors.
    async fn chat_stream(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<ChatStream, Report<LlmServiceError>>;
}

/// Factory for creating [`LlmService`] instances.
///
/// Each call to [`create`](LlmServiceFactory::create) produces a fresh service.
/// The factory is `Clone + Send + Sync` — wrap in `Arc` for sharing.
pub trait LlmServiceFactory: Send + Sync + std::fmt::Debug {
    /// Create a new LLM service instance.
    ///
    /// # Errors
    ///
    /// Returns an error if the factory fails to create a service.
    fn create(&self) -> Result<Box<dyn LlmService>, Report<LlmServiceError>>;

    /// Returns a human-readable name for this factory.
    fn name(&self) -> &str;
}
