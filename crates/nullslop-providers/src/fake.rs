//! Fake LLM service for testing.
//!
//! Supports both plain text streaming and tool-call streaming.
//! Use [`FakeLlmServiceFactory::with_tool_calls`] to simulate tool responses.
//! Use [`FakeLlmServiceFactory::with_tool_loop`] to simulate a multi-turn
//! tool loop where the first call returns tool_use and subsequent calls
//! return end_turn.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use error_stack::Report;
use futures::stream;
use nullslop_protocol::tool::ToolCall;
use nullslop_protocol::LlmMessage;

use crate::service::{ChatStream, LlmService, LlmServiceFactory, LlmServiceError, ToolStream};
use crate::StreamEvent;

/// Special prompt that triggers multi-turn tool loop behavior.
///
/// When the last user message contains this string, the fake service
/// returns a tool_use response on the first call and a text-only response
/// on subsequent calls.
pub const TOOL_LOOP_TRIGGER: &str = "__tool_loop_test__";

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
    /// Shared call counter for multi-turn tool loop simulation.
    ///
    /// When set, the first call with the trigger prompt returns tool_use,
    /// and subsequent calls return end_turn text.
    tool_loop_call_count: Option<Arc<AtomicUsize>>,
    /// Tool calls to use on the first call of a tool loop.
    tool_loop_first_tool_calls: Vec<ToolCall>,
    /// Text tokens to use on subsequent calls of a tool loop.
    tool_loop_subsequent_tokens: Vec<String>,
}

impl FakeLlmServiceFactory {
    /// Create a factory whose services yield the given tokens (text only).
    #[must_use]
    pub fn new(tokens: Vec<String>) -> Self {
        Self {
            tokens,
            tool_calls: vec![],
            tool_loop_call_count: None,
            tool_loop_first_tool_calls: vec![],
            tool_loop_subsequent_tokens: vec![],
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
            tool_loop_call_count: None,
            tool_loop_first_tool_calls: vec![],
            tool_loop_subsequent_tokens: vec![],
        }
    }

    /// Create a factory that simulates a multi-turn tool loop.
    ///
    /// When the last user message contains [`TOOL_LOOP_TRIGGER`], the fake
    /// returns a tool_use response (with the given tool calls and tokens) on
    /// the first call, and a text-only response (with the subsequent tokens)
    /// on the second call. This simulates the LLM calling a tool, receiving
    /// results, and then producing a final text response.
    ///
    /// When the last user message does not contain the trigger, behaves like
    /// [`FakeLlmServiceFactory::new`] with the `tokens` parameter.
    #[must_use]
    pub fn with_tool_loop(
        tokens: Vec<String>,
        first_tool_calls: Vec<ToolCall>,
        subsequent_tokens: Vec<String>,
    ) -> Self {
        Self {
            tokens,
            tool_calls: vec![],
            tool_loop_call_count: Some(Arc::new(AtomicUsize::new(0))),
            tool_loop_first_tool_calls: first_tool_calls,
            tool_loop_subsequent_tokens: subsequent_tokens,
        }
    }

    /// Returns the number of tool loop calls made so far.
    ///
    /// Only meaningful when created with [`Self::with_tool_loop`].
    #[must_use]
    pub fn tool_loop_call_count(&self) -> usize {
        self.tool_loop_call_count
            .as_ref()
            .map(|c| c.load(Ordering::SeqCst))
            .unwrap_or(0)
    }
}

impl LlmServiceFactory for FakeLlmServiceFactory {
    fn create(&self) -> Result<Box<dyn LlmService>, Report<LlmServiceError>> {
        Ok(Box::new(FakeLlmService {
            tokens: self.tokens.clone(),
            tool_calls: self.tool_calls.clone(),
            tool_loop_call_count: self.tool_loop_call_count.clone(),
            tool_loop_first_tool_calls: self.tool_loop_first_tool_calls.clone(),
            tool_loop_subsequent_tokens: self.tool_loop_subsequent_tokens.clone(),
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
    /// Shared call counter for multi-turn tool loop simulation.
    tool_loop_call_count: Option<Arc<AtomicUsize>>,
    /// Tool calls for the first call of a tool loop.
    tool_loop_first_tool_calls: Vec<ToolCall>,
    /// Text tokens for subsequent calls of a tool loop.
    tool_loop_subsequent_tokens: Vec<String>,
}

impl FakeLlmService {
    /// Extracts the content of the last user message, if any.
    fn last_user_content(messages: &[LlmMessage]) -> Option<&str> {
        messages.iter().rev().find_map(|msg| match msg {
            LlmMessage::User { content } => Some(content.as_str()),
            _ => None,
        })
    }

    /// Returns true if the messages contain the tool loop trigger.
    fn is_tool_loop_trigger(messages: &[LlmMessage]) -> bool {
        Self::last_user_content(messages)
            .is_some_and(|c| c.contains(TOOL_LOOP_TRIGGER))
    }

    /// Builds a tool_use stream for the first call of a tool loop.
    fn build_tool_loop_first_stream(
        &self,
    ) -> Vec<Result<StreamEvent, Report<LlmServiceError>>> {
        let mut events: Vec<Result<StreamEvent, Report<LlmServiceError>>> = Vec::new();

        for token in &self.tokens {
            events.push(Ok(StreamEvent::Text(token.clone())));
        }

        for (index, tc) in self.tool_loop_first_tool_calls.iter().enumerate() {
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

        events.push(Ok(StreamEvent::Done {
            stop_reason: "tool_use".to_string(),
        }));

        events
    }

    /// Builds a text-only stream for subsequent calls of a tool loop.
    fn build_tool_loop_subsequent_stream(
        &self,
    ) -> Vec<Result<StreamEvent, Report<LlmServiceError>>> {
        let mut events: Vec<Result<StreamEvent, Report<LlmServiceError>>> = Vec::new();

        for token in &self.tool_loop_subsequent_tokens {
            events.push(Ok(StreamEvent::Text(token.clone())));
        }

        events.push(Ok(StreamEvent::Done {
            stop_reason: "end_turn".to_string(),
        }));

        events
    }
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
        messages: Vec<LlmMessage>,
        _tools: Vec<nullslop_protocol::tool::ToolDefinition>,
    ) -> Result<ToolStream, Report<LlmServiceError>> {
        // Check for multi-turn tool loop trigger.
        if let Some(ref counter) = self.tool_loop_call_count {
            if Self::is_tool_loop_trigger(&messages) {
                let call_num = counter.fetch_add(1, Ordering::SeqCst);
                if call_num == 0 {
                    return Ok(Box::pin(stream::iter(self.build_tool_loop_first_stream())));
                } else {
                    return Ok(Box::pin(stream::iter(self.build_tool_loop_subsequent_stream())));
                }
            }
        }

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

    // --- Multi-turn tool loop tests ---

    #[tokio::test]
    async fn tool_loop_first_call_returns_tool_use() {
        // Given a tool loop factory.
        let tool_call = ToolCall {
            id: "call_1".to_string(),
            name: "echo".to_string(),
            arguments: r#"{"input":"hi"}"#.to_string(),
        };
        let factory = FakeLlmServiceFactory::with_tool_loop(
            vec!["Let me check".to_string()],
            vec![tool_call.clone()],
            vec!["Here is the answer".to_string()],
        );

        // When creating a service and streaming with the trigger prompt.
        let service = factory.create().expect("create service");
        let messages = vec![LlmMessage::User {
            content: "__tool_loop_test__".to_string(),
        }];
        let stream = service
            .chat_stream_with_tools(messages, vec![])
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

    #[tokio::test]
    async fn tool_loop_second_call_returns_text_only() {
        // Given a tool loop factory.
        let tool_call = ToolCall {
            id: "call_1".to_string(),
            name: "echo".to_string(),
            arguments: r#"{"input":"hi"}"#.to_string(),
        };
        let factory = FakeLlmServiceFactory::with_tool_loop(
            vec!["Let me check".to_string()],
            vec![tool_call],
            vec!["Here is the answer".to_string()],
        );

        // When creating a service and making two calls with the trigger.
        let service = factory.create().expect("create service");
        let messages = vec![LlmMessage::User {
            content: "__tool_loop_test__".to_string(),
        }];

        // First call — consume the tool_use response.
        let stream = service
            .chat_stream_with_tools(messages.clone(), vec![])
            .await
            .expect("first call");
        let _events: Vec<StreamEvent> = stream.map(|r| r.expect("event")).collect().await;

        // Second call — should return text only.
        let stream = service
            .chat_stream_with_tools(messages, vec![])
            .await
            .expect("second call");
        let events: Vec<StreamEvent> = stream.map(|r| r.expect("event")).collect().await;

        // Then the stream emits subsequent tokens and Done with end_turn.
        assert_eq!(events.len(), 2);
        assert_eq!(
            events[0],
            StreamEvent::Text("Here is the answer".to_string())
        );
        assert_eq!(
            events[1],
            StreamEvent::Done {
                stop_reason: "end_turn".to_string(),
            }
        );
    }

    #[tokio::test]
    async fn tool_loop_ignores_non_trigger_messages() {
        // Given a tool loop factory.
        let factory = FakeLlmServiceFactory::with_tool_loop(
            vec!["normal response".to_string()],
            vec![],
            vec!["subsequent".to_string()],
        );

        // When streaming with a non-trigger message.
        let service = factory.create().expect("create service");
        let messages = vec![LlmMessage::User {
            content: "regular message".to_string(),
        }];
        let stream = service
            .chat_stream_with_tools(messages, vec![])
            .await
            .expect("chat_stream_with_tools");
        let events: Vec<StreamEvent> = stream.map(|r| r.expect("event")).collect().await;

        // Then the stream emits the default tokens (not the tool loop ones).
        assert_eq!(events.len(), 2);
        assert_eq!(
            events[0],
            StreamEvent::Text("normal response".to_string())
        );
        assert_eq!(
            events[1],
            StreamEvent::Done {
                stop_reason: "end_turn".to_string(),
            }
        );
        assert_eq!(factory.tool_loop_call_count(), 0);
    }
}
