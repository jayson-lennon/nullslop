//! `OpenRouter` implementation of LLM service.

use error_stack::{Report, ResultExt};
use futures::StreamExt;
use llm::builder::{LLMBackend, LLMBuilder};
use llm::chat::ChatMessage;

use crate::service::{ChatStream, LlmService, LlmServiceError, LlmServiceFactory};

/// Default model for `OpenRouter`.
const DEFAULT_MODEL: &str = "openai/gpt-oss-120b";

/// An `OpenRouter` API key.
///
/// Newtype wrapper around the API key string to make the type self-documenting.
#[derive(Debug, Clone)]
pub struct ApiKey(String);

impl ApiKey {
    /// Create an API key from an explicit string value.
    #[must_use]
    pub fn new<T>(key: T) -> Self
    where
        T: Into<String>,
    {
        Self(key.into())
    }

    /// Returns the key as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Factory that creates `OpenRouter` LLM service instances.
#[derive(Debug, Clone)]
pub struct OpenRouterLlmServiceFactory {
    /// API key for authenticating with [`OpenRouter`].
    api_key: ApiKey,
    /// Model identifier to use for requests.
    model: String,
}

impl OpenRouterLlmServiceFactory {
    /// Create a new factory with explicit configuration.
    #[must_use]
    pub fn with_key_and_model(api_key: ApiKey, model: String) -> Self {
        Self { api_key, model }
    }

    /// Returns the default model name.
    #[must_use]
    pub fn default_model() -> &'static str {
        DEFAULT_MODEL
    }
}

impl LlmServiceFactory for OpenRouterLlmServiceFactory {
    fn create(&self) -> Result<Box<dyn LlmService>, Report<LlmServiceError>> {
        let built = LLMBuilder::new()
            .backend(LLMBackend::OpenRouter)
            .api_key(self.api_key.as_str())
            .model(&self.model)
            .build();
        let provider = ResultExt::change_context(built, LlmServiceError::Config)?;
        Ok(Box::new(OpenRouterLlmService { provider }))
    }

    fn name(&self) -> &'static str {
        "OpenRouter"
    }
}

/// A single `OpenRouter` streaming session.
struct OpenRouterLlmService {
    /// The underlying LLM provider for streaming chat.
    provider: Box<dyn llm::LLMProvider>,
}

#[async_trait::async_trait]
impl LlmService for OpenRouterLlmService {
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

// NOTE: api_key_from_env_returns_error_when_not_set test removed.
// ApiKey::from_env() was removed — environment should not be read deep
// in the system. Env capture belongs at program startup.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_key_new_creates_key() {
        // Given an explicit key string.
        let key = ApiKey::new("test-key-123");

        // When creating an ApiKey.
        // Then it holds the value.
        assert_eq!(key.as_str(), "test-key-123");
    }

    #[test]
    fn api_key_as_str_returns_inner() {
        // Given an ApiKey.
        let key = ApiKey::new("sk-abc");

        // When calling as_str.
        // Then the inner value is returned.
        assert_eq!(key.as_str(), "sk-abc");
    }

    #[test]
    fn factory_name_is_open_router() {
        // Given an OpenRouter factory.
        let factory =
            OpenRouterLlmServiceFactory::with_key_and_model(ApiKey::new("test"), "model".into());

        // When asking for the name.
        // Then it returns "OpenRouter".
        assert_eq!(factory.name(), "OpenRouter");
    }

    #[test]
    fn with_key_and_model_creates_factory() {
        // Given an API key and model name.
        let factory = OpenRouterLlmServiceFactory::with_key_and_model(
            ApiKey::new("sk-test"),
            "gpt-4".to_owned(),
        );

        // When creating the factory.
        // Then it succeeds and has the correct name.
        assert_eq!(factory.name(), "OpenRouter");
    }
}
