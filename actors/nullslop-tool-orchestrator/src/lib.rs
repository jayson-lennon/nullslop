//! Tool orchestrator actor — dispatches tool calls and aggregates batch results.
//!
//! This actor maintains a registry of available tools (both built-in and
//! actor-provided), dispatches [`ExecuteToolBatch`] requests, and emits
//! [`ToolBatchCompleted`] when all calls in a batch finish.
//!
//! Built-in tools (`echo`, `get_time`, `file_read`, `file_write`) are registered at
//! activation and executed via spawned tokio tasks. Actor-provided tools
//! are routed via [`ExecuteTool`] commands on the bus.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use nullslop_actor::{Actor, ActorContext, ActorEnvelope, SystemMessage};
use nullslop_protocol::tool::{
    ExecuteTool, ExecuteToolBatch, RegisterTools, ToolBatchCompleted, ToolCall, ToolDefinition,
    ToolExecutionCompleted, ToolResult, ToolsRegistered,
};
use nullslop_protocol::{Command, Event, SessionId};

/// A boxed future returned by built-in tool execute functions.
type BoxedToolFuture = Pin<Box<dyn Future<Output = ToolResult> + Send>>;

/// How a tool is registered and executed.
enum ToolRegistration {
    /// A built-in tool executed directly by the orchestrator.
    Builtin {
        definition: ToolDefinition,
        execute: fn(ToolCall) -> BoxedToolFuture,
    },
    /// An actor-provided tool routed via [`ExecuteTool`] command.
    Actor {
        definition: ToolDefinition,
        provider: String,
    },
}

impl std::fmt::Debug for ToolRegistration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Builtin { definition, .. } => f
                .debug_struct("Builtin")
                .field("name", &definition.name)
                .finish_non_exhaustive(),
            Self::Actor {
                definition,
                provider,
            } => f
                .debug_struct("Actor")
                .field("name", &definition.name)
                .field("provider", provider)
                .finish(),
        }
    }
}

/// Tracks pending tool calls within a batch.
struct PendingBatch {
    /// Number of tool calls still awaiting results.
    remaining: usize,
    /// Collected results so far.
    results: Vec<ToolResult>,
}

/// Direct message type for the tool orchestrator actor.
///
/// Currently unused — the orchestrator only responds to bus commands and events.
pub enum ToolOrchestratorDirectMsg {}

/// Tool orchestrator actor.
///
/// Subscribes to [`RegisterTools`] and [`ExecuteToolBatch`] commands, and
/// [`ToolExecutionCompleted`] events. Dispatches tool calls to the appropriate
/// handler and aggregates results into batch completion events.
pub struct ToolOrchestratorActor {
    /// Tool name → registration info.
    tools: HashMap<String, ToolRegistration>,
    /// Session ID → pending batch tracker.
    pending: HashMap<SessionId, PendingBatch>,
}

impl Actor for ToolOrchestratorActor {
    type Message = ToolOrchestratorDirectMsg;

    fn activate(ctx: &mut ActorContext) -> Self {
        ctx.subscribe_command::<RegisterTools>();
        ctx.subscribe_command::<ExecuteToolBatch>();
        ctx.subscribe_event::<ToolExecutionCompleted>();

        let mut actor = Self {
            tools: HashMap::new(),
            pending: HashMap::new(),
        };

        let builtins = builtin_tools();
        let definitions: Vec<ToolDefinition> = builtins.iter().map(|(d, _)| d.clone()).collect();

        for (def, execute_fn) in builtins {
            let name = def.name.clone();
            actor.tools.insert(
                name,
                ToolRegistration::Builtin {
                    definition: def,
                    execute: execute_fn,
                },
            );
        }

        // Announce built-in tools so the LLM actor can cache them.
        if let Err(e) = ctx.send_event(Event::ToolsRegistered {
            payload: ToolsRegistered {
                provider: "builtin".to_owned(),
                definitions,
            },
        }) {
            tracing::warn!(err = ?e, "failed to emit ToolsRegistered for built-in tools");
        }

        actor
    }

    async fn handle(&mut self, msg: ActorEnvelope<ToolOrchestratorDirectMsg>, ctx: &ActorContext) {
        match msg {
            ActorEnvelope::Command(command) => self.handle_command(&command, ctx),
            ActorEnvelope::Event(event) => self.handle_event(&event, ctx),
            ActorEnvelope::System(SystemMessage::ApplicationShuttingDown) => {
                ctx.announce_shutdown_completed();
            }
            ActorEnvelope::System(SystemMessage::ApplicationReady) => {
                ctx.announce_started();
            }
            ActorEnvelope::Direct(_) | ActorEnvelope::Shutdown => {}
        }
    }

    async fn shutdown(self) {}
}

impl ToolOrchestratorActor {
    /// Dispatches incoming commands to the appropriate handler.
    fn handle_command(&mut self, command: &Command, ctx: &ActorContext) {
        match command {
            Command::RegisterTools { payload } => {
                self.handle_register_tools(&payload.provider, &payload.definitions, ctx);
            }
            Command::ExecuteToolBatch { payload } => {
                self.handle_execute_tool_batch(
                    payload.session_id.clone(),
                    payload.tool_calls.clone(),
                    ctx,
                );
            }
            _ => {}
        }
    }

    /// Dispatches incoming events to the appropriate handler.
    fn handle_event(&mut self, event: &Event, ctx: &ActorContext) {
        match event {
            Event::ToolExecutionCompleted { payload } => {
                self.handle_tool_execution_completed(
                    payload.session_id.clone(),
                    payload.result.clone(),
                    ctx,
                );
            }
            _ => {}
        }
    }

    /// Stores actor-provided tools and emits a [`ToolsRegistered`] event.
    fn handle_register_tools(
        &mut self,
        provider: &str,
        definitions: &[ToolDefinition],
        ctx: &ActorContext,
    ) {
        for def in definitions {
            let name = def.name.clone();
            self.tools.insert(
                name,
                ToolRegistration::Actor {
                    definition: def.clone(),
                    provider: provider.to_owned(),
                },
            );
        }

        if let Err(e) = ctx.send_event(Event::ToolsRegistered {
            payload: ToolsRegistered {
                provider: provider.to_owned(),
                definitions: definitions.to_vec(),
            },
        }) {
            tracing::warn!(err = ?e, "failed to emit ToolsRegistered event");
        }
    }

    /// Dispatches each tool call and tracks the pending batch.
    fn handle_execute_tool_batch(
        &mut self,
        session_id: SessionId,
        tool_calls: Vec<ToolCall>,
        ctx: &ActorContext,
    ) {
        let count = tool_calls.len();

        if count == 0 {
            if let Err(e) = ctx.send_event(Event::ToolBatchCompleted {
                payload: ToolBatchCompleted {
                    session_id,
                    results: vec![],
                },
            }) {
                tracing::warn!(err = ?e, "failed to emit empty ToolBatchCompleted");
            }
            return;
        }

        self.pending.insert(
            session_id.clone(),
            PendingBatch {
                remaining: count,
                results: Vec::with_capacity(count),
            },
        );

        for tool_call in tool_calls {
            self.dispatch_tool_call(session_id.clone(), tool_call, ctx);
        }
    }

    /// Dispatches a single tool call to the appropriate handler.
    fn dispatch_tool_call(&self, session_id: SessionId, tool_call: ToolCall, ctx: &ActorContext) {
        match self.tools.get(&tool_call.name) {
            Some(ToolRegistration::Builtin { execute, .. }) => {
                let sink = ctx.sink();
                let execute_fn = *execute;

                tokio::spawn(async move {
                    let result = execute_fn(tool_call).await;
                    if let Err(e) = sink.send_event(Event::ToolExecutionCompleted {
                        payload: ToolExecutionCompleted { session_id, result },
                    }) {
                        tracing::warn!(
                            err = ?e,
                            "builtin tool failed to send ToolExecutionCompleted"
                        );
                    }
                });
            }
            Some(ToolRegistration::Actor { provider, .. }) => {
                if let Err(e) = ctx.send_command(Command::ExecuteTool {
                    payload: ExecuteTool {
                        session_id,
                        tool_call,
                    },
                }) {
                    tracing::warn!(
                        err = ?e,
                        provider = %provider,
                        "failed to send ExecuteTool command"
                    );
                }
            }
            None => {
                let call_id = tool_call.id.clone();
                let call_name = tool_call.name.clone();
                let result = ToolResult {
                    tool_call_id: call_id,
                    name: call_name,
                    content: format!("unknown tool: {}", tool_call.name),
                    success: false,
                };

                if let Err(e) = ctx.send_event(Event::ToolExecutionCompleted {
                    payload: ToolExecutionCompleted { session_id, result },
                }) {
                    tracing::warn!(
                        err = ?e,
                        "failed to send unknown-tool ToolExecutionCompleted"
                    );
                }
            }
        }
    }

    /// Aggregates a tool result into the pending batch.
    ///
    /// When all calls in a batch have completed, emits [`ToolBatchCompleted`].
    fn handle_tool_execution_completed(
        &mut self,
        session_id: SessionId,
        result: ToolResult,
        ctx: &ActorContext,
    ) {
        let Some(batch) = self.pending.get_mut(&session_id) else {
            tracing::warn!(
                session_id = ?session_id,
                "received ToolExecutionCompleted for unknown session"
            );
            return;
        };

        batch.results.push(result);
        batch.remaining -= 1;

        if batch.remaining == 0 {
            // unwrap: we just checked the entry exists above.
            let results = self
                .pending
                .remove(&session_id)
                .map(|b| b.results)
                .unwrap_or_default();

            if let Err(e) = ctx.send_event(Event::ToolBatchCompleted {
                payload: ToolBatchCompleted {
                    session_id,
                    results,
                },
            }) {
                tracing::warn!(err = ?e, "failed to emit ToolBatchCompleted");
            }
        }
    }

    /// Returns a reference to the tool registration for the given name.
    #[cfg(test)]
    fn get_tool(&self, name: &str) -> Option<&ToolRegistration> {
        self.tools.get(name)
    }
}

// ---------------------------------------------------------------------------
// Built-in tools
// ---------------------------------------------------------------------------

/// Returns the built-in tool definitions and their execute functions.
fn builtin_tools() -> Vec<(ToolDefinition, fn(ToolCall) -> BoxedToolFuture)> {
    vec![
        (
            echo_definition(),
            execute_echo as fn(ToolCall) -> BoxedToolFuture,
        ),
        (
            get_time_definition(),
            execute_get_time as fn(ToolCall) -> BoxedToolFuture,
        ),
        (
            file_read_definition(),
            execute_file_read as fn(ToolCall) -> BoxedToolFuture,
        ),
        (
            file_write_definition(),
            execute_file_write as fn(ToolCall) -> BoxedToolFuture,
        ),
    ]
}

fn echo_definition() -> ToolDefinition {
    ToolDefinition {
        name: "echo".to_owned(),
        description: "Echoes the input text back as the result.".to_owned(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "input": {
                    "type": "string",
                    "description": "Text to echo back"
                }
            },
            "required": ["input"]
        }),
    }
}

fn get_time_definition() -> ToolDefinition {
    ToolDefinition {
        name: "get_time".to_owned(),
        description: "Returns the current date and time in UTC.".to_owned(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {}
        }),
    }
}

fn file_read_definition() -> ToolDefinition {
    ToolDefinition {
        name: "file_read".to_owned(),
        description: "Reads the contents of a file from disk.".to_owned(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to read"
                }
            },
            "required": ["path"]
        }),
    }
}

/// Executes the `echo` built-in tool.
fn execute_echo(call: ToolCall) -> BoxedToolFuture {
    Box::pin(async move {
        match serde_json::from_str::<serde_json::Value>(&call.arguments) {
            Ok(args) => {
                let input = args.get("input").and_then(|v| v.as_str()).unwrap_or("");
                ToolResult {
                    tool_call_id: call.id,
                    name: call.name,
                    content: input.to_owned(),
                    success: true,
                }
            }
            Err(e) => ToolResult {
                tool_call_id: call.id,
                name: call.name,
                content: format!("failed to parse arguments: {e}"),
                success: false,
            },
        }
    })
}

/// Executes the `get_time` built-in tool.
fn execute_get_time(call: ToolCall) -> BoxedToolFuture {
    Box::pin(async move {
        let now = jiff::Zoned::now();
        ToolResult {
            tool_call_id: call.id,
            name: call.name,
            content: now.to_string(),
            success: true,
        }
    })
}

fn file_write_definition() -> ToolDefinition {
    ToolDefinition {
        name: "file_write".to_owned(),
        description: "Writes content to a file on disk, creating parent directories as needed."
            .to_owned(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["path", "content"]
        }),
    }
}

/// Executes the `file_read` built-in tool using async I/O.
fn execute_file_read(call: ToolCall) -> BoxedToolFuture {
    Box::pin(async move {
        let path = match serde_json::from_str::<serde_json::Value>(&call.arguments) {
            Ok(args) => args
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned(),
            Err(e) => {
                return ToolResult {
                    tool_call_id: call.id,
                    name: call.name,
                    content: format!("failed to parse arguments: {e}"),
                    success: false,
                };
            }
        };

        match tokio::fs::read_to_string(&path).await {
            Ok(content) => ToolResult {
                tool_call_id: call.id,
                name: call.name,
                content,
                success: true,
            },
            Err(e) => ToolResult {
                tool_call_id: call.id,
                name: call.name,
                content: format!("failed to read file '{path}': {e}"),
                success: false,
            },
        }
    })
}

/// Executes the `file_write` built-in tool using async I/O.
///
/// Creates parent directories if they don't exist. Overwrites the file if it
/// already exists.
fn execute_file_write(call: ToolCall) -> BoxedToolFuture {
    Box::pin(async move {
        let (path, content) = match serde_json::from_str::<serde_json::Value>(&call.arguments) {
            Ok(args) => {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned();
                let content = args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned();
                (path, content)
            }
            Err(e) => {
                return ToolResult {
                    tool_call_id: call.id,
                    name: call.name,
                    content: format!("failed to parse arguments: {e}"),
                    success: false,
                };
            }
        };

        if let Some(parent) = std::path::Path::new(&path).parent() {
            if !parent.as_os_str().is_empty() {
                if let Err(e) = tokio::fs::create_dir_all(parent).await {
                    return ToolResult {
                        tool_call_id: call.id,
                        name: call.name,
                        content: format!("failed to create parent directories for '{path}': {e}"),
                        success: false,
                    };
                }
            }
        }

        match tokio::fs::write(&path, &content).await {
            Ok(()) => ToolResult {
                tool_call_id: call.id,
                name: call.name,
                content: format!("wrote {} bytes to {path}", content.len()),
                success: true,
            },
            Err(e) => ToolResult {
                tool_call_id: call.id,
                name: call.name,
                content: format!("failed to write file '{path}': {e}"),
                success: false,
            },
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;

    use nullslop_actor::MessageSink;
    use nullslop_protocol::tool::{ExecuteToolBatch, RegisterTools};

    /// A message sink that records commands and events for test assertions.
    struct RecordingSink {
        commands: Mutex<Vec<Command>>,
        events: Mutex<Vec<Event>>,
    }

    impl RecordingSink {
        fn new() -> Self {
            Self {
                commands: Mutex::new(Vec::new()),
                events: Mutex::new(Vec::new()),
            }
        }

        fn events(&self) -> Vec<Event> {
            self.events.lock().clone()
        }

        fn take_events(&self) -> Vec<Event> {
            let mut guard = self.events.lock();
            std::mem::take(&mut guard)
        }

        fn clear(&self) {
            self.commands.lock().clear();
            self.events.lock().clear();
        }
    }

    impl MessageSink for RecordingSink {
        fn send_command(&self, command: Command) -> nullslop_actor::SendResult {
            self.commands.lock().push(command);
            Ok(())
        }

        fn send_event(&self, event: Event) -> nullslop_actor::SendResult {
            self.events.lock().push(event);
            Ok(())
        }
    }

    /// Creates a test context backed by a recording sink.
    fn test_context(sink: &std::sync::Arc<RecordingSink>) -> ActorContext {
        ActorContext::new("test-tool-orchestrator", sink.clone())
    }

    /// Extracts ToolBatchCompleted events from a list of events.
    fn find_batch_completed(events: &[Event]) -> Vec<&ToolBatchCompleted> {
        events
            .iter()
            .filter_map(|e| match e {
                Event::ToolBatchCompleted { payload } => Some(payload),
                _ => None,
            })
            .collect()
    }

    /// Extracts ToolExecutionCompleted events from a list of events.
    fn find_execution_completed(events: &[Event]) -> Vec<&ToolExecutionCompleted> {
        events
            .iter()
            .filter_map(|e| match e {
                Event::ToolExecutionCompleted { payload } => Some(payload),
                _ => None,
            })
            .collect()
    }

    // --- Activation tests ---

    #[test]
    fn activate_registers_builtin_tools() {
        // Given a fresh actor context.
        let sink = std::sync::Arc::new(RecordingSink::new());
        let mut ctx = test_context(&sink);

        // When activating the actor.
        let actor = ToolOrchestratorActor::activate(&mut ctx);

        // Then the built-in tools are registered.
        assert!(actor.tools.contains_key("echo"));
        assert!(actor.tools.contains_key("get_time"));
        assert!(actor.tools.contains_key("file_read"));
        assert!(actor.tools.contains_key("file_write"));
    }

    #[test]
    fn activate_emits_tools_registered_for_builtins() {
        // Given a fresh actor context with a recording sink.
        let sink = std::sync::Arc::new(RecordingSink::new());
        let mut ctx = test_context(&sink);

        // When activating the actor.
        let _actor = ToolOrchestratorActor::activate(&mut ctx);

        // Then a ToolsRegistered event was emitted for built-in tools.
        let events = sink.events();
        let tools_registered: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                Event::ToolsRegistered { payload } => Some(payload.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            tools_registered.len(),
            1,
            "expected one ToolsRegistered event"
        );

        let payload = &tools_registered[0];
        assert_eq!(payload.provider, "builtin");
        assert_eq!(payload.definitions.len(), 4);

        let names: Vec<&str> = payload
            .definitions
            .iter()
            .map(|d| d.name.as_str())
            .collect();
        assert!(names.contains(&"echo"), "expected echo tool");
        assert!(names.contains(&"get_time"), "expected get_time tool");
        assert!(names.contains(&"file_read"), "expected file_read tool");
        assert!(names.contains(&"file_write"), "expected file_write tool");
    }

    // --- RegisterTools command tests ---

    #[test]
    fn register_tools_stores_actor_tools() {
        // Given an activated actor.
        let sink = std::sync::Arc::new(RecordingSink::new());
        let mut ctx = test_context(&sink);
        let mut actor = ToolOrchestratorActor::activate(&mut ctx);
        sink.clear();

        let definition = ToolDefinition {
            name: "web_search".to_owned(),
            description: "Search the web".to_owned(),
            parameters: serde_json::json!({"type": "object", "properties": {}}),
        };

        // When registering an actor-provided tool.
        let cmd = Command::RegisterTools {
            payload: RegisterTools {
                provider: "web-actor".to_owned(),
                definitions: vec![definition],
            },
        };
        actor.handle_command(&cmd, &ctx);

        // Then the tool is stored in the registry.
        let reg = actor
            .get_tool("web_search")
            .expect("tool should be registered");
        match reg {
            ToolRegistration::Actor { provider, .. } => {
                assert_eq!(provider, "web-actor");
            }
            other => panic!("expected Actor registration, got {other:?}"),
        }
    }

    #[test]
    fn register_tools_emits_tools_registered_event() {
        // Given an activated actor.
        let sink = std::sync::Arc::new(RecordingSink::new());
        let mut ctx = test_context(&sink);
        let mut actor = ToolOrchestratorActor::activate(&mut ctx);
        sink.clear();

        let definition = ToolDefinition {
            name: "web_search".to_owned(),
            description: "Search the web".to_owned(),
            parameters: serde_json::json!({"type": "object", "properties": {}}),
        };

        // When registering tools.
        let cmd = Command::RegisterTools {
            payload: RegisterTools {
                provider: "web-actor".to_owned(),
                definitions: vec![definition.clone()],
            },
        };
        actor.handle_command(&cmd, &ctx);

        // Then a ToolsRegistered event is emitted.
        let events = sink.events();
        assert_eq!(events.len(), 1);
        match &events[0] {
            Event::ToolsRegistered { payload } => {
                assert_eq!(payload.provider, "web-actor");
                assert_eq!(payload.definitions.len(), 1);
                assert_eq!(payload.definitions[0].name, "web_search");
            }
            other => panic!("expected ToolsRegistered, got {other:?}"),
        }
    }

    // --- Built-in tool execution tests ---

    #[tokio::test]
    async fn execute_builtin_echo_tool() {
        // Given an echo tool call.
        let call = ToolCall {
            id: "call_1".to_owned(),
            name: "echo".to_owned(),
            arguments: r#"{"input":"hello world"}"#.to_owned(),
        };

        // When executing the echo tool.
        let result = execute_echo(call).await;

        // Then the result contains the echoed input.
        assert_eq!(result.tool_call_id, "call_1");
        assert_eq!(result.name, "echo");
        assert_eq!(result.content, "hello world");
        assert!(result.success);
    }

    #[tokio::test]
    async fn execute_builtin_echo_tool_returns_error_on_bad_json() {
        // Given an echo tool call with invalid JSON.
        let call = ToolCall {
            id: "call_2".to_owned(),
            name: "echo".to_owned(),
            arguments: "not json".to_owned(),
        };

        // When executing the echo tool.
        let result = execute_echo(call).await;

        // Then the result indicates failure.
        assert_eq!(result.tool_call_id, "call_2");
        assert!(!result.success);
        assert!(result.content.contains("failed to parse arguments"));
    }

    #[tokio::test]
    async fn execute_builtin_get_time_tool() {
        // Given a get_time tool call.
        let call = ToolCall {
            id: "call_3".to_owned(),
            name: "get_time".to_owned(),
            arguments: "{}".to_owned(),
        };

        // When executing the get_time tool.
        let result = execute_get_time(call).await;

        // Then the result has non-empty content.
        assert_eq!(result.tool_call_id, "call_3");
        assert!(result.success);
        assert!(!result.content.is_empty());
    }

    #[tokio::test]
    async fn execute_builtin_file_read_tool() {
        // Given a temp file with known content.
        let dir = tempfile::tempdir().expect("create temp dir");
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "file contents here").expect("write temp file");

        let call = ToolCall {
            id: "call_4".to_owned(),
            name: "file_read".to_owned(),
            arguments: serde_json::json!({
                "path": file_path.to_string_lossy()
            })
            .to_string(),
        };

        // When executing the file_read tool.
        let result = execute_file_read(call).await;

        // Then the result contains the file contents.
        assert_eq!(result.tool_call_id, "call_4");
        assert!(result.success);
        assert_eq!(result.content, "file contents here");
    }

    #[tokio::test]
    async fn execute_builtin_file_read_tool_returns_error_on_missing_file() {
        // Given a file_read call for a nonexistent file.
        let call = ToolCall {
            id: "call_5".to_owned(),
            name: "file_read".to_owned(),
            arguments: serde_json::json!({
                "path": "/nonexistent/path/to/file.txt"
            })
            .to_string(),
        };

        // When executing the file_read tool.
        let result = execute_file_read(call).await;

        // Then the result indicates failure.
        assert_eq!(result.tool_call_id, "call_5");
        assert!(!result.success);
        assert!(result.content.contains("failed to read file"));
    }

    // --- Batch execution tests ---

    #[tokio::test]
    async fn execute_batch_with_single_builtin_tool() {
        // Given an activated actor.
        let sink = std::sync::Arc::new(RecordingSink::new());
        let mut ctx = test_context(&sink);
        let mut actor = ToolOrchestratorActor::activate(&mut ctx);
        sink.clear();

        let session_id = SessionId::new();

        // When executing a batch with one echo call.
        let cmd = Command::ExecuteToolBatch {
            payload: ExecuteToolBatch {
                session_id: session_id.clone(),
                tool_calls: vec![ToolCall {
                    id: "call_1".to_owned(),
                    name: "echo".to_owned(),
                    arguments: r#"{"input":"hello"}"#.to_owned(),
                }],
            },
        };
        actor.handle_command(&cmd, &ctx);

        // Then a ToolExecutionCompleted event arrives from the spawned task.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let events = sink.take_events();
        let completed = find_execution_completed(&events);
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].result.content, "hello");
        assert!(completed[0].result.success);

        // When feeding the completion event back to the actor.
        let completion_event = Event::ToolExecutionCompleted {
            payload: ToolExecutionCompleted {
                session_id: session_id.clone(),
                result: completed[0].result.clone(),
            },
        };
        actor.handle_event(&completion_event, &ctx);

        // Then a ToolBatchCompleted event is emitted.
        let events = sink.events();
        let batch_completed = find_batch_completed(&events);
        assert_eq!(batch_completed.len(), 1);
        assert_eq!(batch_completed[0].results.len(), 1);
        assert_eq!(batch_completed[0].results[0].content, "hello");
    }

    #[tokio::test]
    async fn execute_batch_with_multiple_builtin_tools() {
        // Given an activated actor.
        let sink = std::sync::Arc::new(RecordingSink::new());
        let mut ctx = test_context(&sink);
        let mut actor = ToolOrchestratorActor::activate(&mut ctx);
        sink.clear();

        let session_id = SessionId::new();

        // When executing a batch with two echo calls.
        let cmd = Command::ExecuteToolBatch {
            payload: ExecuteToolBatch {
                session_id: session_id.clone(),
                tool_calls: vec![
                    ToolCall {
                        id: "call_a".to_owned(),
                        name: "echo".to_owned(),
                        arguments: r#"{"input":"first"}"#.to_owned(),
                    },
                    ToolCall {
                        id: "call_b".to_owned(),
                        name: "echo".to_owned(),
                        arguments: r#"{"input":"second"}"#.to_owned(),
                    },
                ],
            },
        };
        actor.handle_command(&cmd, &ctx);

        // Then two ToolExecutionCompleted events arrive.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let events = sink.take_events();
        let completed = find_execution_completed(&events);
        assert_eq!(completed.len(), 2);

        // When feeding the first completion back.
        actor.handle_event(
            &Event::ToolExecutionCompleted {
                payload: ToolExecutionCompleted {
                    session_id: session_id.clone(),
                    result: completed[0].result.clone(),
                },
            },
            &ctx,
        );

        // Then no batch completed yet (one remaining).
        let events = sink.take_events();
        assert!(find_batch_completed(&events).is_empty());

        // When feeding the second completion back.
        actor.handle_event(
            &Event::ToolExecutionCompleted {
                payload: ToolExecutionCompleted {
                    session_id: session_id.clone(),
                    result: completed[1].result.clone(),
                },
            },
            &ctx,
        );

        // Then ToolBatchCompleted is emitted with both results.
        let events = sink.events();
        let batch_completed = find_batch_completed(&events);
        assert_eq!(batch_completed.len(), 1);
        assert_eq!(batch_completed[0].results.len(), 2);
    }

    #[test]
    fn execute_batch_with_unknown_tool_returns_error_result() {
        // Given an activated actor.
        let sink = std::sync::Arc::new(RecordingSink::new());
        let mut ctx = test_context(&sink);
        let mut actor = ToolOrchestratorActor::activate(&mut ctx);
        sink.clear();

        let session_id = SessionId::new();

        // When executing a batch with an unknown tool name.
        let cmd = Command::ExecuteToolBatch {
            payload: ExecuteToolBatch {
                session_id: session_id.clone(),
                tool_calls: vec![ToolCall {
                    id: "call_x".to_owned(),
                    name: "nonexistent_tool".to_owned(),
                    arguments: "{}".to_owned(),
                }],
            },
        };
        actor.handle_command(&cmd, &ctx);

        // Then a ToolExecutionCompleted event with an error is emitted synchronously.
        let events = sink.events();
        let completed = find_execution_completed(&events);
        assert_eq!(completed.len(), 1);
        assert!(!completed[0].result.success);
        assert!(completed[0].result.content.contains("unknown tool"));

        // When feeding the error result back.
        actor.handle_event(
            &Event::ToolExecutionCompleted {
                payload: ToolExecutionCompleted {
                    session_id: session_id.clone(),
                    result: completed[0].result.clone(),
                },
            },
            &ctx,
        );

        // Then ToolBatchCompleted is emitted with the error result.
        let events = sink.events();
        let batch_completed = find_batch_completed(&events);
        assert_eq!(batch_completed.len(), 1);
        assert_eq!(batch_completed[0].results.len(), 1);
        assert!(!batch_completed[0].results[0].success);
    }

    #[test]
    fn execute_batch_with_no_tool_calls_emits_empty_batch_completed() {
        // Given an activated actor.
        let sink = std::sync::Arc::new(RecordingSink::new());
        let mut ctx = test_context(&sink);
        let mut actor = ToolOrchestratorActor::activate(&mut ctx);
        sink.clear();

        let session_id = SessionId::new();

        // When executing a batch with no tool calls.
        let cmd = Command::ExecuteToolBatch {
            payload: ExecuteToolBatch {
                session_id: session_id.clone(),
                tool_calls: vec![],
            },
        };
        actor.handle_command(&cmd, &ctx);

        // Then an empty ToolBatchCompleted is emitted immediately.
        let events = sink.events();
        let batch_completed = find_batch_completed(&events);
        assert_eq!(batch_completed.len(), 1);
        assert!(batch_completed[0].results.is_empty());
    }

    #[tokio::test]
    async fn execute_builtin_file_write_tool() {
        // Given a temp directory.
        let dir = tempfile::tempdir().expect("create temp dir");
        let file_path = dir.path().join("output.txt");

        let call = ToolCall {
            id: "call_fw1".to_owned(),
            name: "file_write".to_owned(),
            arguments: serde_json::json!({
                "path": file_path.to_string_lossy(),
                "content": "hello from file_write"
            })
            .to_string(),
        };

        // When executing the file_write tool.
        let result = execute_file_write(call).await;

        // Then the result indicates success.
        assert_eq!(result.tool_call_id, "call_fw1");
        assert!(result.success, "expected success, got: {}", result.content);
        assert!(result.content.contains("wrote 21 bytes"));

        // And the file contains the written content.
        let content = std::fs::read_to_string(&file_path).expect("read written file");
        assert_eq!(content, "hello from file_write");
    }

    #[tokio::test]
    async fn execute_builtin_file_write_tool_creates_parent_dirs() {
        // Given a temp directory.
        let dir = tempfile::tempdir().expect("create temp dir");
        let file_path = dir.path().join("nested").join("deep").join("file.txt");

        let call = ToolCall {
            id: "call_fw2".to_owned(),
            name: "file_write".to_owned(),
            arguments: serde_json::json!({
                "path": file_path.to_string_lossy(),
                "content": "nested content"
            })
            .to_string(),
        };

        // When executing the file_write tool.
        let result = execute_file_write(call).await;

        // Then the result indicates success.
        assert_eq!(result.tool_call_id, "call_fw2");
        assert!(result.success, "expected success, got: {}", result.content);

        // And the file was created with parent directories.
        let content = std::fs::read_to_string(&file_path).expect("read written file");
        assert_eq!(content, "nested content");
    }

    #[tokio::test]
    async fn execute_builtin_file_write_tool_overwrites_existing_file() {
        // Given a temp file with existing content.
        let dir = tempfile::tempdir().expect("create temp dir");
        let file_path = dir.path().join("existing.txt");
        std::fs::write(&file_path, "old content").expect("write existing file");

        let call = ToolCall {
            id: "call_fw3".to_owned(),
            name: "file_write".to_owned(),
            arguments: serde_json::json!({
                "path": file_path.to_string_lossy(),
                "content": "new content"
            })
            .to_string(),
        };

        // When executing the file_write tool.
        let result = execute_file_write(call).await;

        // Then the result indicates success.
        assert!(result.success);

        // And the file was overwritten.
        let content = std::fs::read_to_string(&file_path).expect("read overwritten file");
        assert_eq!(content, "new content");
    }

    #[tokio::test]
    async fn execute_builtin_file_write_tool_returns_error_on_bad_json() {
        // Given a file_write call with invalid JSON.
        let call = ToolCall {
            id: "call_fw4".to_owned(),
            name: "file_write".to_owned(),
            arguments: "not json".to_owned(),
        };

        // When executing the file_write tool.
        let result = execute_file_write(call).await;

        // Then the result indicates failure.
        assert_eq!(result.tool_call_id, "call_fw4");
        assert!(!result.success);
        assert!(result.content.contains("failed to parse arguments"));
    }

    #[test]
    fn tool_execution_completed_for_unknown_session_is_ignored() {
        // Given an activated actor with no pending batches.
        let sink = std::sync::Arc::new(RecordingSink::new());
        let mut ctx = test_context(&sink);
        let mut actor = ToolOrchestratorActor::activate(&mut ctx);
        sink.clear();

        let unknown_session = SessionId::new();

        // When receiving a ToolExecutionCompleted for an unknown session.
        let event = Event::ToolExecutionCompleted {
            payload: ToolExecutionCompleted {
                session_id: unknown_session,
                result: ToolResult {
                    tool_call_id: "call_0".to_owned(),
                    name: "echo".to_owned(),
                    content: "orphan".to_owned(),
                    success: true,
                },
            },
        };
        actor.handle_event(&event, &ctx);

        // Then no batch completed event is emitted.
        let events = sink.events();
        let batch_completed = find_batch_completed(&events);
        assert!(batch_completed.is_empty());
    }
}
