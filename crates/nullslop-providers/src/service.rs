//! LLM service trait and error types.

use error_stack::Report;
use futures::stream;
use futures::stream::Stream;
use futures::StreamExt;
use nullslop_protocol::tool::ToolDefinition;
use nullslop_protocol::LlmMessage;
use std::pin::Pin;
use wherror::Error;

use crate::StreamEvent;

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

/// A streaming LLM chat response (text tokens only).
pub type ChatStream = Pin<Box<dyn Stream<Item = Result<String, Report<LlmServiceError>>> + Send>>;

/// A streaming LLM chat response with tool support.
pub type ToolStream = Pin<Box<dyn Stream<Item = Result<StreamEvent, Report<LlmServiceError>>> + Send>>;

/// Trait for a single LLM streaming chat session.
///
/// Use [`LlmServiceFactory`] to create instances.
#[async_trait::async_trait]
pub trait LlmService: Send + Sync {
    /// Start a streaming chat completion (text only).
    ///
    /// Returns a stream of text tokens. The stream ends when the LLM finishes
    /// generating or errors.
    async fn chat_stream(
        &self,
        messages: Vec<LlmMessage>,
    ) -> Result<ChatStream, Report<LlmServiceError>>;

    /// Start a streaming chat completion with tool support.
    ///
    /// Returns a stream of [`StreamEvent`] variants. When `tools` is non-empty,
    /// the stream may include tool call events. The default implementation
    /// delegates to [`chat_stream`](LlmService::chat_stream), wrapping text
    /// tokens as [`StreamEvent::Text`] and appending a terminal
    /// [`StreamEvent::Done`].
    async fn chat_stream_with_tools(
        &self,
        messages: Vec<LlmMessage>,
        tools: Vec<ToolDefinition>,
    ) -> Result<ToolStream, Report<LlmServiceError>> {
        let _ = tools; // Default: no tool support, ignore tool definitions.
        let text_stream = self.chat_stream(messages).await?;
        let events = text_stream.map(|result| result.map(StreamEvent::Text));
        let done = stream::once(async {
            Ok(StreamEvent::Done {
                stop_reason: "end_turn".to_string(),
            })
        });
        Ok(Box::pin(events.chain(done)))
    }
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
