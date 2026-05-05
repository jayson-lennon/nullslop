//! Cucumber `World` wrapping real actors for actor-level integration testing.
//!
//! The [`ActorWorld`] creates an [`AppCore`] with [`InMemoryActorHost`] hosting
//! the real LLM and tool orchestrator actors, backed by a fake LLM factory
//! that simulates multi-turn tool loop behavior.

use std::sync::Arc;
use std::time::{Duration, Instant};

use cucumber::World;
use nullslop_actor::{Actor, ActorContext, ActorEnvelope, ActorRef};
use nullslop_actor_host::{InMemoryActorHost, spawn_actor};
use nullslop_component::AppState;
use nullslop_component_core::Bus;
use nullslop_core::{ActorMessageSink, AppCore, AppMsg, TickResult};
use nullslop_llm::LlmActor;
use nullslop_protocol::provider::SendToLlmProvider;
use nullslop_protocol::tool::ToolCall;
use nullslop_providers::{FakeLlmServiceFactory, LlmServiceFactoryService, TOOL_LOOP_TRIGGER};
use nullslop_services::Services;
use nullslop_tool_orchestrator::ToolOrchestratorActor;

/// Maximum time the test will wait for actor messages to settle.
const SETTLE_TIMEOUT: Duration = Duration::from_secs(5);

/// Sleep duration between ticks.
const TICK_INTERVAL: Duration = Duration::from_millis(50);

/// Number of consecutive idle ticks before declaring settled.
const IDLE_TICKS_TO_SETTLE: usize = 3;

/// Cucumber world wrapping real actors for integration testing.
///
/// Created fresh for each scenario. The LLM actor and tool orchestrator
/// actor are running in-memory, communicating through the bus.
#[derive(World)]
#[world(init = Self::new_actor_world)]
pub struct ActorWorld {
    /// The application core (bus, state, message channel).
    pub core: AppCore,
    /// Runtime services.
    #[allow(dead_code)]
    pub services: Services,
}

impl std::fmt::Debug for ActorWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActorWorld")
            .field("state", &self.core.state)
            .finish_non_exhaustive()
    }
}

impl ActorWorld {
    /// Creates a new world with real actors backed by the tool loop fake factory.
    fn new_actor_world() -> Self {
        let rt = Box::leak(Box::new(
            tokio::runtime::Runtime::new().expect("test runtime"),
        ));
        let handle = rt.handle().clone();

        let tool_call = ToolCall {
            id: "call_echo_1".to_string(),
            name: "echo".to_string(),
            arguments: r#"{"input":"hello"}"#.to_string(),
        };
        let fake_factory = FakeLlmServiceFactory::with_tool_loop(
            vec!["Let me check".to_string()],
            vec![tool_call],
            vec!["The answer is done".to_string()],
        );
        let llm_service = LlmServiceFactoryService::new(Arc::new(fake_factory));

        let (core, services) = create_actor_core(&handle, llm_service);
        Self { core, services }
    }

    /// Submits a command to the core's message channel.
    pub fn submit_command(&self, cmd: nullslop_protocol::Command) {
        self.core.submit_command(cmd);
    }

    /// Runs the core tick loop until settled.
    pub fn run_until_settled(&mut self) {
        let start = Instant::now();
        let mut consecutive_idle = 0;

        loop {
            let TickResult {
                should_quit,
                did_work,
            } = self.core.tick();

            if should_quit {
                return;
            }

            if did_work {
                consecutive_idle = 0;
            } else {
                consecutive_idle += 1;
                if consecutive_idle >= IDLE_TICKS_TO_SETTLE {
                    return;
                }
            }

            if start.elapsed() > SETTLE_TIMEOUT {
                eprintln!("actor world timed out after {:?}", SETTLE_TIMEOUT);
                return;
            }

            std::thread::sleep(TICK_INTERVAL);
        }
    }

    /// Returns a read guard to the application state.
    pub fn state(&self) -> nullslop_core::StateReadGuard<'_> {
        self.core.state.read()
    }
}

/// Creates an `AppCore` with the LLM actor and tool orchestrator actor
/// running via `InMemoryActorHost`.
fn create_actor_core(
    handle: &tokio::runtime::Handle,
    llm_service: LlmServiceFactoryService,
) -> (AppCore, Services) {
    let (sender, receiver) = kanal::unbounded::<AppMsg>();
    let sink = Arc::new(ActorMessageSink::new(sender.clone()));

    // Create tool orchestrator actor.
    let (orch_tx, orch_rx) =
        kanal::unbounded::<ActorEnvelope<nullslop_tool_orchestrator::ToolOrchestratorDirectMsg>>();
    let orch_ref = ActorRef::new(orch_tx);
    let mut orch_ctx = ActorContext::new("tool-orchestrator", sink.clone());
    let orch_actor = ToolOrchestratorActor::activate(&mut orch_ctx);
    let orch_result = spawn_actor(
        "tool-orchestrator",
        orch_actor,
        &orch_ref,
        orch_rx,
        orch_ctx,
        handle,
    );

    // Create LLM actor with fake factory.
    let (llm_tx, llm_rx) = kanal::unbounded::<ActorEnvelope<nullslop_llm::LlmDirectMsg>>();
    let llm_ref = ActorRef::new(llm_tx);
    let mut llm_ctx = ActorContext::new("llm-streaming", sink.clone());
    llm_ctx.set_data(llm_service.clone());
    let llm_actor = LlmActor::activate(&mut llm_ctx);
    let llm_result = spawn_actor(
        "llm-streaming",
        llm_actor,
        &llm_ref,
        llm_rx,
        llm_ctx,
        handle,
    );

    let host =
        InMemoryActorHost::from_actors_with_handle(vec![orch_result, llm_result], handle.clone());
    let host_arc: Arc<dyn nullslop_actor_host::ActorHost> = Arc::new(host);

    let services = nullslop_services::test_services::TestServices::builder()
        .handle(handle.clone())
        .actor_host(host_arc.clone())
        .llm_service(llm_service)
        .build();

    let mut core = AppCore {
        bus: Bus::new(),
        state: nullslop_core::State::new(AppState::default()),
        services: services.clone(),
        sender,
        receiver,
        actor_host: Some(nullslop_actor_host::ActorHostService::new(host_arc)),
    };

    let mut registry = nullslop_component::AppUiRegistry::new();
    nullslop_component::register_all(&mut core.bus, &mut registry);

    (core, services)
}

// ---------------------------------------------------------------------------
// Step definitions
// ---------------------------------------------------------------------------

#[cucumber::given(expr = "a fresh actor world with the tool loop fake")]
fn given_fresh_actor_world(_world: &mut ActorWorld) {}

#[cucumber::when(expr = "I submit SendToLlmProvider with the tool loop trigger")]
fn when_submit_tool_loop_trigger(world: &mut ActorWorld) {
    let session_id = world.state().active_session.clone();
    world.submit_command(nullslop_protocol::Command::SendToLlmProvider {
        payload: SendToLlmProvider {
            session_id,
            messages: vec![nullslop_protocol::LlmMessage::User {
                content: TOOL_LOOP_TRIGGER.to_string(),
            }],
            provider_id: None,
        },
    });
    world.run_until_settled();
}

#[cucumber::then(expr = "the chat history should contain at least {int} entries")]
fn then_chat_history_at_least(world: &mut ActorWorld, min: u64) {
    let count = world.state().active_session().history().len();
    assert!(
        count >= min as usize,
        "expected at least {min} history entries, got {count}"
    );
}

#[cucumber::then(expr = "the session should be idle")]
fn then_session_idle(world: &mut ActorWorld) {
    assert!(
        world.state().active_session().is_idle(),
        "expected session to be idle"
    );
}
