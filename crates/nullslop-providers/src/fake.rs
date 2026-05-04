//! Fake LLM service for testing.
//!
//! Supports both plain text streaming and tool-call streaming.
//! Use [`FakeLlmServiceFactory::with_tool_calls`] to simulate tool responses.

use error_stack::Report;
use futures::stream;
use nullslop_protocol::tool::ToolCall;
use nullslop_protocol::LlmMessage;

use crate::service::{ChatStream, LlmService, LlmServiceError, LlmServiceFactory, ToolStream};
use crate::StreamEvent;

/// Factory that creates fake LLM service instances.
///
/// Each service yields the tokens the factory was configured with.
/// Optionally emits tool call events before the text tokens.
/// Use this in tests to avoid hitting real LLM backends.
#[derive(Debug, Clone)]
pub struct FakeLlmServiceFactory {
    /// Tokens each created service will yield.
    tokens: Vec<String>,
    /// Tool calls to emit during streaming.
    tool_calls: Vec<ToolCall>,
}

impl FakeLlmServiceFactory {
    /// Create a factory whose services yield the given tokens (text only).
    #[must_use]
    pub fn new(tokens: Vec<String>) -> Self {
        Self {
            tokens,
            tool_calls: vec![],
        }
    }

    /// Create a factory whose services yield text tokens and tool call events.
    ///
    /// The stream emits: text tokens → tool call events → Done.
    /// The stop reason is `"tool_use"` when tool calls are configured,
    /// `"end_turn"` otherwise.
    #[must_use]
    pub fn with_tool_calls(tokens: Vec<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            tokens,
            tool_calls,
        }
    }
}

impl LlmServiceFactory for FakeLlmServiceFactory {
    fn create(&self) -> Result<Box<dyn LlmService>, Report<LlmServiceError>> {
        Ok(Box::new(FakeLlmService {
            tokens: self.tokens.clone(),
            tool_calls: self.tool_calls.clone(),
        }))
    }

    fn name(&self) -> &'static str {
        "FakeLlm"
    }
}

/// A fake LLM service that yields preconfigured tokens and tool calls.
struct FakeLlmService {
    /// Tokens to yield during streaming.
    tokens: Vec<String>,
    /// Tool calls to emit during tool streaming.
    tool_calls: Vec<ToolCall>,
}

#[async_trait::async_trait]
impl LlmService for FakeLlmService {
    async fn chat_stream(
        &self,
        _messages: Vec<LlmMessage>,
    ) -> Result<ChatStream, Report<LlmServiceError>> {
        let tokens = self.tokens.clone();
        let stream: ChatStream = Box::pin(stream::iter(tokens.into_iter().map(Ok)));
        Ok(stream)
    }

    async fn chat_stream_with_tools(
        &self,
        _messages: Vec<LlmMessage>,
        _tools: Vec<nullslop_protocol::tool::ToolDefinition>,
    ) -> Result<ToolStream, Report<LlmServiceError>> {
        let mut events: Vec<Result<StreamEvent, Report<LlmServiceError>>> = Vec::new();

        // Emit text tokens.
        for token in &self.tokens {
            events.push(Ok(StreamEvent::Text(token.clone())));
        }

        if self.tool_calls.is_empty() {
            // No tool calls — just text and Done(end_turn).
            events.push(Ok(StreamEvent::Done {
                stop_reason: "end_turn".to_string(),
            }));
        } else {
            // Emit tool call events.
            for (index, tc) in self.tool_calls.iter().enumerate() {
                events.push(Ok(StreamEvent::ToolUseStart {
                    index,
                    id: tc.id.clone(),
                    name: tc.name.clone(),
                }));
                events.push(Ok(StreamEvent::ToolUseInputDelta {
                    index,
                    partial_json: tc.arguments.clone(),
                }));
                events.push(Ok(StreamEvent::ToolUseComplete {
                    index,
                    tool_call: tc.clone(),
                }));
            }

            // Terminal event.
            events.push(Ok(StreamEvent::Done {
                stop_reason: "tool_use".to_string(),
            }));
        }

        Ok(Box::pin(stream::iter(events)))
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;

    use super::*;

    #[test]
    fn fake_factory_creates_service() {
        // Given a fake factory.
        let factory = FakeLlmServiceFactory::new(vec!["hello".to_owned()]);

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
            "Hello".to_owned(),
            " world".to_owned(),
            "!".to_owned(),
        ]);

        // When creating a service and streaming.
        let service = factory.create().expect("create service");
        let stream = service.chat_stream(vec![]).await.expect("chat_stream");
        let tokens: Vec<String> = StreamExt::map(stream, |r| r.expect("token"))
            .collect()
            .await;

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

    #[tokio::test]
    async fn fake_service_yields_text_events_and_done() {
        // Given a fake factory with no tool calls.
        let factory = FakeLlmServiceFactory::new(vec!["Hello".to_string()]);

        // When streaming with tools.
        let service = factory.create().expect("create service");
        let stream = service
            .chat_stream_with_tools(vec![], vec![])
            .await
            .expect("chat_stream_with_tools");
        let events: Vec<StreamEvent> = stream.map(|r| r.expect("event")).collect().await;

        // Then text tokens are wrapped in StreamEvent::Text and stream ends with Done.
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], StreamEvent::Text("Hello".to_string()));
        assert_eq!(
            events[1],
            StreamEvent::Done {
                stop_reason: "end_turn".to_string(),
            }
        );
    }

    #[tokio::test]
    async fn fake_service_yields_tool_call_events_and_done() {
        // Given a fake factory with tool calls.
        let tool_call = ToolCall {
            id: "call_1".to_string(),
            name: "echo".to_string(),
            arguments: r#"{"input":"hi"}"#.to_string(),
        };
        let factory =
            FakeLlmServiceFactory::with_tool_calls(vec!["Let me check".to_string()], vec![
                tool_call.clone(),
            ]);

        // When streaming with tools.
        let service = factory.create().expect("create service");
        let stream = service
            .chat_stream_with_tools(vec![], vec![])
            .await
            .expect("chat_stream_with_tools");
        let events: Vec<StreamEvent> = stream.map(|r| r.expect("event")).collect().await;

        // Then the stream emits text, tool events, and Done with tool_use.
        assert!(matches!(&events[0], StreamEvent::Text(t) if t == "Let me check"));
        assert!(matches!(&events[1], StreamEvent::ToolUseStart { index: 0, .. }));
        assert!(matches!(&events[2], StreamEvent::ToolUseInputDelta { index: 0, .. }));
        assert!(matches!(&events[3], StreamEvent::ToolUseComplete { index: 0, .. }));
        assert_eq!(
            events[4],
            StreamEvent::Done {
                stop_reason: "tool_use".to_string(),
            }
        );
    }
}
