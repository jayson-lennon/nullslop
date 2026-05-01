//! Fake LLM service for testing.

use error_stack::Report;
use futures::stream;
use llm::chat::ChatMessage;

use crate::service::{ChatStream, LlmService, LlmServiceError, LlmServiceFactory};

/// Factory that creates fake LLM service instances.
///
/// Each service yields the tokens the factory was configured with.
/// Use this in tests to avoid hitting real LLM backends.
#[derive(Debug, Clone)]
pub struct FakeLlmServiceFactory {
    tokens: Vec<String>,
}

impl FakeLlmServiceFactory {
    /// Create a factory whose services yield the given tokens.
    #[must_use]
    pub fn new(tokens: Vec<String>) -> Self {
        Self { tokens }
    }
}

impl LlmServiceFactory for FakeLlmServiceFactory {
    fn create(&self) -> Result<Box<dyn LlmService>, Report<LlmServiceError>> {
        Ok(Box::new(FakeLlmService {
            tokens: self.tokens.clone(),
        }))
    }

    fn name(&self) -> &'static str {
        "FakeLlm"
    }
}

/// A fake LLM service that yields preconfigured tokens.
struct FakeLlmService {
    tokens: Vec<String>,
}

#[async_trait::async_trait]
impl LlmService for FakeLlmService {
    async fn chat_stream(
        &self,
        _messages: Vec<ChatMessage>,
    ) -> Result<ChatStream, Report<LlmServiceError>> {
        let tokens = self.tokens.clone();
        let stream: ChatStream = Box::pin(stream::iter(tokens.into_iter().map(Ok)));
        Ok(stream)
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;

    use super::*;

    #[test]
    fn fake_factory_creates_service() {
        // Given a fake factory.
        let factory = FakeLlmServiceFactory::new(vec!["hello".to_string()]);

        // When creating a service.
        let result = factory.create();

        // Then a boxed service is returned.
        assert!(result.is_ok());
    }

    #[test]
    fn fake_factory_name() {
        // Given a fake factory.
        let factory = FakeLlmServiceFactory::new(vec![]);

        // When asking for the name.
        // Then it returns "FakeLlm".
        assert_eq!(factory.name(), "FakeLlm");
    }

    #[tokio::test]
    async fn fake_service_yields_configured_tokens() {
        // Given a fake factory with specific tokens.
        let factory = FakeLlmServiceFactory::new(vec![
            "Hello".to_string(),
            " world".to_string(),
            "!".to_string(),
        ]);

        // When creating a service and streaming.
        let service = factory.create().expect("create service");
        let stream = service.chat_stream(vec![]).await.expect("chat_stream");
        let tokens: Vec<String> = stream.map(|r| r.expect("token")).collect().await;

        // Then the tokens match the configured list.
        assert_eq!(tokens, vec!["Hello", " world", "!"]);
    }

    #[tokio::test]
    async fn fake_service_empty_tokens() {
        // Given a fake factory with no tokens.
        let factory = FakeLlmServiceFactory::new(vec![]);

        // When creating a service and streaming.
        let service = factory.create().expect("create service");
        let stream = service.chat_stream(vec![]).await.expect("chat_stream");
        let tokens: Vec<String> = stream.map(|r| r.expect("token")).collect().await;

        // Then no tokens are produced.
        assert!(tokens.is_empty());
    }
}
