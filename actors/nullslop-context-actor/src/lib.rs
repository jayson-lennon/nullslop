//! Prompt assembly actor — assembles LLM-ready prompts from chat history.
//!
//! Subscribes to [`AssemblePrompt`] and [`SwitchPromptStrategy`] commands,
//! runs the configured strategy for each session, and emits [`PromptAssembled`]
//! and [`PromptStrategySwitched`] events when complete.
//!
//! Unknown sessions are automatically initialized with [`PassthroughStrategy`].
//! Strategy switching uses a [`StrategyFactory`] injected via [`ActorContext`] data.

use std::collections::HashMap;

use nullslop_actor::{Actor, ActorContext, ActorEnvelope, SystemMessage};
use nullslop_context::{AssemblyContext, DefaultStrategyFactory, PromptAssembly, StrategyFactory};
use nullslop_protocol::context::{AssemblePrompt, PromptAssembled, PromptStrategySwitched, RestoreStrategyState, SwitchPromptStrategy};
use nullslop_protocol::tool::ToolsRegistered;
use nullslop_protocol::{Event, SessionId, ToolDefinition};

/// Direct message type for the prompt assembly actor (unused for now).
pub enum ContextDirectMsg {}

/// The prompt assembly actor.
pub struct PromptAssemblyActor {
    strategies: HashMap<SessionId, Box<dyn PromptAssembly>>,
    tool_definitions: HashMap<String, ToolDefinition>,
    factory: Option<Box<dyn StrategyFactory>>,
}

impl Actor for PromptAssemblyActor {
    type Message = ContextDirectMsg;

    fn activate(ctx: &mut ActorContext) -> Self {
        ctx.subscribe_command::<AssemblePrompt>();
        ctx.subscribe_command::<SwitchPromptStrategy>();
        ctx.subscribe_command::<RestoreStrategyState>();
        ctx.subscribe_event::<ToolsRegistered>();
        let factory = ctx.take_data::<Box<dyn StrategyFactory>>()
            .unwrap_or_else(|| Box::new(DefaultStrategyFactory));
        Self {
            strategies: HashMap::new(),
            tool_definitions: HashMap::new(),
            factory: Some(factory),
        }
    }

    async fn handle(
        &mut self,
        msg: ActorEnvelope<Self::Message>,
        ctx: &ActorContext,
    ) {
        match msg {
            ActorEnvelope::Command(cmd) => {
                self.handle_command(&cmd, ctx).await;
            }
            ActorEnvelope::Event(evt) => {
                self.handle_event(&evt);
            }
            ActorEnvelope::System(SystemMessage::ApplicationReady) => {
                ctx.announce_started();
            }
            ActorEnvelope::System(SystemMessage::ApplicationShuttingDown) => {
                ctx.announce_shutdown_completed();
            }
            ActorEnvelope::Direct(_) | ActorEnvelope::Shutdown => {}
        }
    }

    async fn shutdown(self) {}
}

impl PromptAssemblyActor {
    async fn handle_command(&mut self, cmd: &nullslop_protocol::Command, ctx: &ActorContext) {
        match cmd {
            nullslop_protocol::Command::AssemblePrompt { payload } => {
                self.on_assemble_prompt(payload, ctx).await;
            }
            nullslop_protocol::Command::SwitchPromptStrategy { payload } => {
                self.on_switch_prompt_strategy(payload, ctx);
            }
            nullslop_protocol::Command::RestoreStrategyState { payload } => {
                self.on_restore_strategy_state(payload);
            }
            _ => {}
        }
    }

    fn handle_event(&mut self, evt: &nullslop_protocol::Event) {
        match evt {
            Event::ToolsRegistered { payload } => {
                self.on_tools_registered(payload);
            }
            _ => {}
        }
    }

    fn ensure_strategy(&mut self, session_id: &SessionId) {
        if !self.strategies.contains_key(session_id) {
            self.strategies.insert(session_id.clone(), Box::new(nullslop_context::PassthroughStrategy));
        }
    }

    async fn on_assemble_prompt(&mut self, cmd: &AssemblePrompt, ctx: &ActorContext) {
        let session_id = cmd.session_id.clone();
        self.ensure_strategy(&session_id);
        let tools: Vec<ToolDefinition> = cmd
            .tools
            .iter()
            .cloned()
            .chain(
                self.tool_definitions
                    .values()
                    .cloned()
                    .filter(|td| !cmd.tools.iter().any(|t| t.name == td.name)),
            )
            .collect();
        let strategy = self.strategies.get(&session_id).expect("strategy was just ensured");
        let context = AssemblyContext {
            history: &cmd.history,
            tools: &tools,
            model_name: &cmd.model_name,
            session_id: &session_id,
        };
        let result = match strategy.assemble(&context).await {
            Ok(assembled) => assembled,
            Err(e) => {
                tracing::error!("prompt assembly failed: {e:?}");
                return;
            }
        };
        let _ = ctx.send_event(Event::PromptAssembled {
            payload: PromptAssembled {
                session_id,
                system_prompt: result.system_prompt,
                messages: result.messages,
            },
        });
    }

    fn on_switch_prompt_strategy(&mut self, cmd: &SwitchPromptStrategy, ctx: &ActorContext) {
        let factory = match self.factory.as_ref() {
            Some(f) => f,
            None => {
                tracing::error!("no strategy factory available");
                return;
            }
        };
        match factory.create(&cmd.strategy_id) {
            Ok(new_strategy) => {
                self.strategies.insert(cmd.session_id.clone(), new_strategy);
                let _ = ctx.send_event(Event::PromptStrategySwitched {
                    payload: PromptStrategySwitched {
                        session_id: cmd.session_id.clone(),
                        strategy_id: cmd.strategy_id.clone(),
                    },
                });
            }
            Err(e) => {
                tracing::error!("failed to create strategy '{}': {e:?}", cmd.strategy_id);
            }
        }
    }

    fn on_tools_registered(&mut self, evt: &ToolsRegistered) {
        for def in &evt.definitions {
            self.tool_definitions.insert(def.name.clone(), def.clone());
        }
    }

    fn on_restore_strategy_state(&mut self, cmd: &RestoreStrategyState) {
        // Stub: accept the restore command gracefully.
        // The full implementation will deserialize the blob into
        // strategy-specific state and attach it to the strategy.
        tracing::debug!(
            session_id = ?cmd.session_id,
            strategy_id = %cmd.strategy_id,
            "received RestoreStrategyState (stub: no-op)"
        );
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use nullslop_actor::{ActorContext, MessageSink};
    use nullslop_protocol::ChatEntry;
    use nullslop_protocol::PromptStrategyId;

    use super::*;

    #[derive(Debug)]
    struct RecordingSink {
        events: std::sync::Mutex<Vec<Event>>,
    }

    impl RecordingSink {
        fn new() -> Self {
            Self {
                events: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn events(&self) -> Vec<Event> {
            self.events.lock().expect("lock").clone()
        }
    }

    impl MessageSink for RecordingSink {
        fn send_command(
            &self,
            _command: nullslop_protocol::Command,
        ) -> nullslop_actor::SendResult {
            Ok(())
        }

        fn send_event(
            &self,
            event: Event,
        ) -> nullslop_actor::SendResult {
            self.events.lock().expect("lock").push(event);
            Ok(())
        }
    }

    fn test_context(sink: Arc<RecordingSink>) -> ActorContext {
        ActorContext::new("nullslop-context-actor", sink as Arc<dyn MessageSink>)
    }

    fn find_prompt_assembled(events: &[Event]) -> Option<PromptAssembled> {
        for evt in events {
            if let Event::PromptAssembled { payload } = evt {
                return Some(payload.clone());
            }
        }
        None
    }

    fn find_strategy_switched(events: &[Event]) -> Option<PromptStrategySwitched> {
        for evt in events {
            if let Event::PromptStrategySwitched { payload } = evt {
                return Some(payload.clone());
            }
        }
        None
    }

    #[tokio::test]
    async fn passthrough_assembly_produces_messages() {
        // Given an actor with a fresh context.
        let sink = Arc::new(RecordingSink::new());
        let mut ctx = test_context(sink.clone());
        let mut actor = PromptAssemblyActor::activate(&mut ctx);

        // When sending an AssemblePrompt with history.
        let session_id = SessionId::new();
        let history = vec![ChatEntry::user("hello"), ChatEntry::assistant("hi")];
        let cmd = nullslop_protocol::Command::AssemblePrompt {
            payload: AssemblePrompt {
                session_id: session_id.clone(),
                history,
                tools: vec![],
                model_name: "test".to_owned(),
            },
        };
        actor.handle(ActorEnvelope::Command(cmd), &ctx).await;

        // Then a PromptAssembled event is emitted with the messages.
        let events = sink.events();
        let assembled = find_prompt_assembled(&events);
        assert!(assembled.is_some());
        let assembled = assembled.expect("should have PromptAssembled");
        assert_eq!(assembled.session_id, session_id);
        assert!(assembled.system_prompt.is_none());
        assert_eq!(assembled.messages.len(), 2);
    }

    #[tokio::test]
    async fn unknown_session_gets_passthrough() {
        // Given an actor.
        let sink = Arc::new(RecordingSink::new());
        let mut ctx = test_context(sink.clone());
        let mut actor = PromptAssemblyActor::activate(&mut ctx);

        // When sending an AssemblePrompt for a new session.
        let session_id = SessionId::new();
        let cmd = nullslop_protocol::Command::AssemblePrompt {
            payload: AssemblePrompt {
                session_id: session_id.clone(),
                history: vec![ChatEntry::user("test")],
                tools: vec![],
                model_name: "test".to_owned(),
            },
        };
        actor.handle(ActorEnvelope::Command(cmd), &ctx).await;

        // Then assembly succeeds (auto-initialized with passthrough).
        let events = sink.events();
        assert!(find_prompt_assembled(&events).is_some());
    }

    #[tokio::test]
    async fn tools_registered_caches_definitions() {
        // Given an actor.
        let sink = Arc::new(RecordingSink::new());
        let mut ctx = test_context(sink.clone());
        let mut actor = PromptAssemblyActor::activate(&mut ctx);

        // When receiving a ToolsRegistered event.
        let evt = Event::ToolsRegistered {
            payload: ToolsRegistered {
                provider: "echo-actor".to_owned(),
                definitions: vec![ToolDefinition {
                    name: "echo".to_owned(),
                    description: "echo tool".to_owned(),
                    parameters: serde_json::json!({}),
                }],
            },
        };
        actor.handle(ActorEnvelope::Event(evt), &ctx).await;

        // Then the tool definition is cached.
        assert!(actor.tool_definitions.contains_key("echo"));
    }

    #[tokio::test]
    async fn switch_strategy_replaces_strategy() {
        // Given an actor with an existing session.
        let sink = Arc::new(RecordingSink::new());
        let mut ctx = test_context(sink.clone());
        let mut actor = PromptAssemblyActor::activate(&mut ctx);

        let session_id = SessionId::new();
        // Initialize the session with an assemble.
        let cmd = nullslop_protocol::Command::AssemblePrompt {
            payload: AssemblePrompt {
                session_id: session_id.clone(),
                history: vec![ChatEntry::user("hello")],
                tools: vec![],
                model_name: "test".to_owned(),
            },
        };
        actor.handle(ActorEnvelope::Command(cmd), &ctx).await;
        sink.events().clear();

        // When switching to sliding_window strategy.
        let switch_cmd = nullslop_protocol::Command::SwitchPromptStrategy {
            payload: SwitchPromptStrategy {
                session_id: session_id.clone(),
                strategy_id: PromptStrategyId::sliding_window(),
            },
        };
        actor.handle(ActorEnvelope::Command(switch_cmd), &ctx).await;

        // Then a PromptStrategySwitched event is emitted.
        let events = sink.events();
        let switched = find_strategy_switched(&events);
        assert!(switched.is_some());
        let switched = switched.expect("should have PromptStrategySwitched");
        assert_eq!(switched.session_id, session_id);
        assert_eq!(switched.strategy_id, PromptStrategyId::sliding_window());

        // And the strategy is now sliding_window.
        let strategy = actor.strategies.get(&session_id).expect("should exist");
        assert_eq!(strategy.name(), "sliding_window");
    }

    #[tokio::test]
    async fn switch_strategy_unknown_id_is_ignored() {
        // Given an actor.
        let sink = Arc::new(RecordingSink::new());
        let mut ctx = test_context(sink.clone());
        let mut actor = PromptAssemblyActor::activate(&mut ctx);

        // When switching to an unknown strategy.
        let session_id = SessionId::new();
        let switch_cmd = nullslop_protocol::Command::SwitchPromptStrategy {
            payload: SwitchPromptStrategy {
                session_id: session_id.clone(),
                strategy_id: PromptStrategyId::new("nonexistent"),
            },
        };
        actor.handle(ActorEnvelope::Command(switch_cmd), &ctx).await;

        // Then no event is emitted and no strategy is stored.
        let events = sink.events();
        assert!(find_strategy_switched(&events).is_none());
        assert!(!actor.strategies.contains_key(&session_id));
    }

    #[tokio::test]
    async fn sliding_window_strategy_limits_output() {
        // Given an actor with a session switched to sliding_window.
        let sink = Arc::new(RecordingSink::new());
        let mut ctx = test_context(sink.clone());
        let mut actor = PromptAssemblyActor::activate(&mut ctx);

        let session_id = SessionId::new();

        // Switch to sliding window.
        let switch_cmd = nullslop_protocol::Command::SwitchPromptStrategy {
            payload: SwitchPromptStrategy {
                session_id: session_id.clone(),
                strategy_id: PromptStrategyId::sliding_window(),
            },
        };
        actor.handle(ActorEnvelope::Command(switch_cmd), &ctx).await;
        sink.events().clear();

        // When assembling with more than 50 entries.
        let mut history = Vec::new();
        for i in 0..60 {
            history.push(ChatEntry::user(format!("msg {i}")));
        }
        let cmd = nullslop_protocol::Command::AssemblePrompt {
            payload: AssemblePrompt {
                session_id: session_id.clone(),
                history,
                tools: vec![],
                model_name: "test".to_owned(),
            },
        };
        actor.handle(ActorEnvelope::Command(cmd), &ctx).await;

        // Then only the last 50 entries are in the output.
        let events = sink.events();
        let assembled = find_prompt_assembled(&events).expect("should have PromptAssembled");
        assert_eq!(assembled.messages.len(), 50);
    }

    #[tokio::test]
    async fn token_budget_strategy_limits_output() {
        // Given an actor with a session switched to token_budget.
        let sink = Arc::new(RecordingSink::new());
        let mut ctx = test_context(sink.clone());
        let mut actor = PromptAssemblyActor::activate(&mut ctx);

        let session_id = SessionId::new();

        // Switch to token_budget.
        let switch_cmd = nullslop_protocol::Command::SwitchPromptStrategy {
            payload: SwitchPromptStrategy {
                session_id: session_id.clone(),
                strategy_id: PromptStrategyId::token_budget(),
            },
        };
        actor.handle(ActorEnvelope::Command(switch_cmd), &ctx).await;
        sink.events().clear();

        // When assembling with many large entries that exceed the 8192 token budget.
        let mut history = Vec::new();
        for _ in 0..100 {
            // Each entry: 400 chars / 4 + 1 = 101 tokens. 100 entries = 10,100 tokens.
            history.push(ChatEntry::user("a".repeat(400)));
        }
        let cmd = nullslop_protocol::Command::AssemblePrompt {
            payload: AssemblePrompt {
                session_id: session_id.clone(),
                history,
                tools: vec![],
                model_name: "test".to_owned(),
            },
        };
        actor.handle(ActorEnvelope::Command(cmd), &ctx).await;

        // Then the output is trimmed and a system prompt is set.
        let events = sink.events();
        let assembled = find_prompt_assembled(&events).expect("should have PromptAssembled");
        assert!(assembled.messages.len() < 100);
        assert!(assembled.system_prompt.is_some());
    }

    #[tokio::test]
    async fn compaction_strategy_limits_output() {
        // Given an actor with a session switched to compaction.
        let sink = Arc::new(RecordingSink::new());
        let mut ctx = test_context(sink.clone());
        let mut actor = PromptAssemblyActor::activate(&mut ctx);

        let session_id = SessionId::new();

        // Switch to compaction.
        let switch_cmd = nullslop_protocol::Command::SwitchPromptStrategy {
            payload: SwitchPromptStrategy {
                session_id: session_id.clone(),
                strategy_id: PromptStrategyId::compaction(),
            },
        };
        actor.handle(ActorEnvelope::Command(switch_cmd), &ctx).await;
        sink.events().clear();

        // When assembling with many entries that exceed the 8192 token budget.
        let mut history = Vec::new();
        for _ in 0..100 {
            history.push(ChatEntry::user("a".repeat(400)));
        }
        let cmd = nullslop_protocol::Command::AssemblePrompt {
            payload: AssemblePrompt {
                session_id: session_id.clone(),
                history,
                tools: vec![],
                model_name: "test".to_owned(),
            },
        };
        actor.handle(ActorEnvelope::Command(cmd), &ctx).await;

        // Then the output is trimmed with a compaction system prompt.
        let events = sink.events();
        let assembled = find_prompt_assembled(&events).expect("should have PromptAssembled");
        assert!(assembled.messages.len() < 100);
        assert_eq!(
            assembled.system_prompt.as_deref(),
            Some("Context was compacted to fit within the token budget. Earlier conversation history was summarized.")
        );
    }

    #[tokio::test]
    async fn restore_strategy_state_accepted() {
        // Given an actor.
        let sink = Arc::new(RecordingSink::new());
        let mut ctx = test_context(sink.clone());
        let mut actor = PromptAssemblyActor::activate(&mut ctx);

        // When sending a RestoreStrategyState command.
        let session_id = SessionId::new();
        let cmd = nullslop_protocol::Command::RestoreStrategyState {
            payload: RestoreStrategyState {
                session_id: session_id.clone(),
                strategy_id: PromptStrategyId::compaction(),
                blob: serde_json::json!({"compaction_count": 5}),
            },
        };
        // Then the command is handled without error (no panic).
        actor.handle(ActorEnvelope::Command(cmd), &ctx).await;

        // And no events are emitted (stub is a no-op).
        let events = sink.events();
        assert!(events.is_empty());
    }
}
