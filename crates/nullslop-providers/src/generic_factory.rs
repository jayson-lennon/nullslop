//! Generic LLM service factory — works for any `LLMBackend`.
//!
//! [`GenericLlmServiceFactory`] stores a provider configuration and a resolved
//! API key. It builds the appropriate `LlmService` on each call. The API key
//! is provided at construction time (resolved from env vars at startup), not
//! read from the environment.

use error_stack::{Report, ResultExt as _};
use futures::StreamExt;
use llm::builder::{LLMBackend, LLMBuilder};
use llm::chat::ChatMessage;

use crate::service::{ChatStream, LlmService, LlmServiceError, LlmServiceFactory};

/// Generic factory that builds an LLM service from a provider config.
///
/// Stores the backend, model, optional base URL, and a resolved API key.
/// The key is provided at construction time — environment access belongs
/// at application startup, not in the factory.
#[derive(Debug, Clone)]
pub struct GenericLlmServiceFactory {
    /// Display name for this factory.
    name: String,
    /// Which LLM backend to use.
    backend: LLMBackend,
    /// Model identifier.
    model: String,
    /// Optional base URL override (for local providers).
    base_url: Option<String>,
    /// Resolved API key (empty string if not required).
    api_key: String,
}

impl GenericLlmServiceFactory {
    /// Create a new generic factory from resolved config values.
    #[must_use]
    pub fn new(
        name: String,
        backend: LLMBackend,
        model: String,
        base_url: Option<String>,
        api_key: String,
    ) -> Self {
        Self {
            name,
            backend,
            model,
            base_url,
            api_key,
        }
    }
}

impl LlmServiceFactory for GenericLlmServiceFactory {
    fn create(&self) -> Result<Box<dyn LlmService>, Report<LlmServiceError>> {
        let mut builder = LLMBuilder::new()
            .backend(self.backend.clone())
            .model(&self.model);

        if let Some(ref url) = self.base_url {
            builder = builder.base_url(url);
        }

        if !self.api_key.is_empty() {
            builder = builder.api_key(&self.api_key);
        }

        let provider = builder
            .build()
            .change_context(LlmServiceError::Config)
            .attach("failed to build LLM provider")?;

        Ok(Box::new(GenericLlmService { provider }))
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// A single generic streaming session.
struct GenericLlmService {
    /// The underlying LLM provider for streaming chat.
    provider: Box<dyn llm::LLMProvider>,
}

#[async_trait::async_trait]
impl LlmService for GenericLlmService {
    async fn chat_stream(
        &self,
        messages: Vec<ChatMessage>,
    ) -> Result<ChatStream, Report<LlmServiceError>> {
        let stream = self
            .provider
            .chat_stream(&messages)
            .await
            .change_context(LlmServiceError::Provider)?;
        let mapped: ChatStream = Box::pin(StreamExt::map(
            stream,
            |token_result: Result<String, llm::error::LLMError>| {
                token_result.change_context(LlmServiceError::Provider)
            },
        ));
        Ok(mapped)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_returns_configured_name() {
        // Given a generic factory with a custom name.
        let factory = GenericLlmServiceFactory::new(
            "my-provider".to_owned(),
            LLMBackend::OpenRouter,
            "gpt-4".to_owned(),
            None,
            String::new(),
        );

        // When asking for the name.
        // Then it returns the configured name.
        assert_eq!(factory.name(), "my-provider");
    }
}
