//! No-provider factory — streams a help message when no provider is configured.
//!
//! Used as the initial factory when no provider is available at startup.
//! The streamed message explains how to configure providers.

use error_stack::Report;
use futures::stream;
use nullslop_protocol::LlmMessage;

use crate::service::{ChatStream, LlmService, LlmServiceError, LlmServiceFactory};

/// The message streamed when no provider is configured.
const HELP_MESSAGE: &str = "No LLM provider is configured. To get started:\n\
    \n\
    1. Edit ~/.config/nullslop/providers.toml\n\
    2. Uncomment one of the provider entries\n\
    3. Set the API key environment variable referenced by api_key_env\n\
    4. Open the provider picker (press p) and select a provider\n\
    \n\
    For local providers like Ollama, no API key is needed — just uncomment and select.";

/// Sentinel provider ID used when no real provider is configured.
pub const NO_PROVIDER_ID: &str = "__no_provider__";

/// Factory that creates a service which streams the help message.
#[derive(Debug, Clone)]
pub struct NoProvidersAvailableFactory;

impl LlmServiceFactory for NoProvidersAvailableFactory {
    fn create(&self) -> Result<Box<dyn LlmService>, Report<LlmServiceError>> {
        Ok(Box::new(NoProvidersAvailableService))
    }

    fn name(&self) -> &'static str {
        "NoProvidersAvailable"
    }
}

/// Service that streams the help message.
struct NoProvidersAvailableService;

#[async_trait::async_trait]
impl LlmService for NoProvidersAvailableService {
    async fn chat_stream(
        &self,
        _messages: Vec<LlmMessage>,
    ) -> Result<ChatStream, Report<LlmServiceError>> {
        let tokens = tokenize(HELP_MESSAGE);
        let stream: ChatStream = Box::pin(stream::iter(tokens.into_iter().map(Ok)));
        Ok(stream)
    }
}

/// Splits text into word-by-word tokens with leading spaces preserved.
fn tokenize(text: &str) -> Vec<String> {
    text.split(' ')
        .enumerate()
        .map(|(i, word)| {
            if i == 0 {
                word.to_owned()
            } else {
                format!(" {word}")
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;

    use super::*;

    #[test]
    fn factory_name_is_no_providers_available() {
        // Given a NoProvidersAvailableFactory.
        let factory = NoProvidersAvailableFactory;

        // When asking for the name.
        // Then it returns "NoProvidersAvailable".
        assert_eq!(factory.name(), "NoProvidersAvailable");
    }

    #[test]
    fn factory_creates_service() {
        // Given a NoProvidersAvailableFactory.
        let factory = NoProvidersAvailableFactory;

        // When creating a service.
        let result = factory.create();

        // Then it succeeds.
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn service_streams_help_message() {
        // Given a NoProvidersAvailableFactory.
        let factory = NoProvidersAvailableFactory;

        // When creating a service and streaming.
        let service = factory.create().expect("create service");
        let stream = service.chat_stream(vec![]).await.expect("chat_stream");
        let tokens: Vec<String> = StreamExt::map(stream, |r| r.expect("token"))
            .collect()
            .await;

        // Then the concatenated output matches HELP_MESSAGE.
        let output: String = tokens.into_iter().collect();
        assert_eq!(output, HELP_MESSAGE);
    }
}
