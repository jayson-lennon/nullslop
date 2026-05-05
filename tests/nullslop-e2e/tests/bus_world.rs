//! Cucumber `World` wrapping a [`Bus`] for handler-level integration testing.
//!
//! The [`BusWorld`] creates a fresh bus with all component handlers registered,
//! an [`AppState`], and [`Services`] — enabling command/event submission and
//! state assertions through natural-language step definitions.

use std::sync::Arc;

use cucumber::World;
use nullslop_actor_host::FakeActorHost;
use nullslop_component::AppState;
use nullslop_component_core::Bus;
use nullslop_protocol as npr;
use nullslop_providers::{
    ApiKeys, ApiKeysService, ConfigStorageService, InMemoryConfigStorage, LlmServiceFactoryService,
    ProviderEntry, ProviderRegistry, ProviderRegistryService, ProvidersConfig,
};
use nullslop_services::Services;

/// Cucumber world wrapping a [`Bus`] for handler-level testing.
///
/// Created fresh for each scenario. All component handlers are registered.
/// Provides methods for submitting commands/events and asserting state.
#[derive(World)]
#[world(init = Self::new_all_handlers)]
pub struct BusWorld {
    /// The message bus with all handlers registered.
    pub bus: Bus<AppState, Services>,
    /// The application state.
    pub state: AppState,
    /// The services instance.
    pub services: Services,
}

impl std::fmt::Debug for BusWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BusWorld")
            .field("state", &self.state)
            .field("services", &self.services)
            .finish_non_exhaustive()
    }
}

impl BusWorld {
    /// Creates a new world with all handlers registered and default services.
    fn new_all_handlers() -> Self {
        let services = Self::default_services();
        let mut bus: Bus<AppState, Services> = Bus::new();
        let mut registry = nullslop_component::AppUiRegistry::new();
        nullslop_component::register_all(&mut bus, &mut registry);
        // Drop registry — we only need bus handlers for these tests.
        drop(registry);
        Self {
            bus,
            state: AppState::default(),
            services,
        }
    }

    /// Creates default test services with no providers.
    fn default_services() -> Services {
        let rt = Box::leak(Box::new(
            tokio::runtime::Runtime::new().expect("test runtime"),
        ));
        let handle = rt.handle().clone();
        let actor_host: Arc<dyn nullslop_actor_host::ActorHost> = Arc::new(FakeActorHost::new());
        let llm = LlmServiceFactoryService::new(Arc::new(
            nullslop_providers::FakeLlmServiceFactory::new(vec![]),
        ));
        let registry = ProviderRegistryService::new(
            ProviderRegistry::from_config(ProvidersConfig {
                providers: vec![],
                aliases: vec![],
                default_provider: None,
            })
            .expect("test registry"),
        );
        let api_keys = ApiKeysService::new(ApiKeys::new());
        let config_storage = ConfigStorageService::new(Arc::new(InMemoryConfigStorage::new()));

        Services::new(handle, actor_host, llm, registry, api_keys, config_storage)
    }

    /// Creates test services with an ollama provider.
    fn services_with_ollama() -> Services {
        let rt = Box::leak(Box::new(
            tokio::runtime::Runtime::new().expect("test runtime"),
        ));
        let handle = rt.handle().clone();
        let config = ProvidersConfig {
            providers: vec![ProviderEntry {
                name: "ollama".to_owned(),
                backend: "ollama".to_owned(),
                models: vec!["llama3".to_owned()],
                base_url: Some("http://localhost:11434".to_owned()),
                api_key_env: None,
                requires_key: false,
            }],
            aliases: vec![],
            default_provider: None,
        };
        let actor_host: Arc<dyn nullslop_actor_host::ActorHost> = Arc::new(FakeActorHost::new());
        let llm = LlmServiceFactoryService::new(Arc::new(
            nullslop_providers::FakeLlmServiceFactory::new(vec![]),
        ));
        let registry =
            ProviderRegistryService::new(ProviderRegistry::from_config(config).expect("registry"));
        let api_keys = ApiKeysService::new(ApiKeys::new());
        let config_storage = ConfigStorageService::new(Arc::new(InMemoryConfigStorage::new()));

        Services::new(handle, actor_host, llm, registry, api_keys, config_storage)
    }

    /// Creates test services with an unavailable (key-required) provider.
    fn services_with_unavailable() -> Services {
        let rt = Box::leak(Box::new(
            tokio::runtime::Runtime::new().expect("test runtime"),
        ));
        let handle = rt.handle().clone();
        let config = ProvidersConfig {
            providers: vec![ProviderEntry {
                name: "openrouter".to_owned(),
                backend: "openrouter".to_owned(),
                models: vec!["gpt-4".to_owned()],
                base_url: None,
                api_key_env: Some("OPENROUTER_API_KEY".to_owned()),
                requires_key: true,
            }],
            aliases: vec![],
            default_provider: None,
        };
        let actor_host: Arc<dyn nullslop_actor_host::ActorHost> = Arc::new(FakeActorHost::new());
        let llm = LlmServiceFactoryService::new(Arc::new(
            nullslop_providers::FakeLlmServiceFactory::new(vec![]),
        ));
        let registry =
            ProviderRegistryService::new(ProviderRegistry::from_config(config).expect("registry"));
        let api_keys = ApiKeysService::new(ApiKeys::new());
        let config_storage = ConfigStorageService::new(Arc::new(InMemoryConfigStorage::new()));

        Services::new(handle, actor_host, llm, registry, api_keys, config_storage)
    }

    /// Submits a command and processes it.
    fn submit_and_process(&mut self, cmd: npr::Command) {
        self.bus.submit_command(cmd);
        self.bus.process_commands(&mut self.state, &self.services);
    }

    /// Submits an event, processes events, then processes any cascaded commands.
    fn submit_event_and_process(&mut self, evt: npr::Event) {
        self.bus.submit_event(evt);
        self.bus.process_events(&mut self.state, &self.services);
        self.bus.process_commands(&mut self.state, &self.services);
    }

    /// Drains processed commands, returning how many match a predicate.
    fn count_processed_commands(&mut self, pred: impl Fn(&npr::Command) -> bool) -> usize {
        self.bus
            .drain_processed_commands()
            .iter()
            .filter(|c| pred(&c.command))
            .count()
    }

    /// Returns true if any processed command matches the predicate.
    fn has_processed_command(&mut self, pred: impl Fn(&npr::Command) -> bool) -> bool {
        self.count_processed_commands(pred) > 0
    }
}

// ---------------------------------------------------------------------------
// Step definitions — Chat Input
// ---------------------------------------------------------------------------

// --- Given steps ---

#[cucumber::given(expr = "a fresh bus with all handlers")]
fn given_fresh_bus_all_handlers(_world: &mut BusWorld) {}

#[cucumber::given(expr = "the active provider is {string}")]
fn given_active_provider_is(world: &mut BusWorld, provider: String) {
    world.state.active_provider = provider;
}

#[cucumber::given(expr = "the input buffer contains {string}")]
fn given_input_buffer_contains(world: &mut BusWorld, text: String) {
    let text = text.replace("\\n", "\n").replace("\\t", "\t");
    world
        .state
        .active_chat_input_mut()
        .replace_all(text);
}

#[cucumber::given(expr = "the app is in {word} mode")]
fn given_app_in_mode(world: &mut BusWorld, mode: String) {
    world.state.mode = parse_mode(&mode);
}

#[cucumber::given(expr = "the session is sending")]
fn given_session_sending(world: &mut BusWorld) {
    world.state.active_session_mut().begin_sending();
}

#[cucumber::given(expr = "the session is streaming")]
fn given_session_streaming(world: &mut BusWorld) {
    world.state.active_session_mut().begin_streaming();
}

#[cucumber::given(expr = "the session has queued message {string}")]
fn given_session_queued_message(world: &mut BusWorld, text: String) {
    world.state.active_session_mut().enqueue_message(text);
}

#[cucumber::given(expr = "the session is idle")]
fn given_session_idle(_world: &mut BusWorld) {}

#[cucumber::given(expr = "the picker selection is {int}")]
fn given_picker_selection_is(world: &mut BusWorld, index: u64) {
    world.state.picker.selection = index as usize;
}

#[cucumber::given(expr = "services with an ollama provider")]
fn given_services_with_ollama(world: &mut BusWorld) {
    world.services = BusWorld::services_with_ollama();
}

#[cucumber::given(expr = "services with an unavailable provider")]
fn given_services_with_unavailable(world: &mut BusWorld) {
    world.services = BusWorld::services_with_unavailable();
}

// --- When steps ---

#[cucumber::when(expr = "I submit InsertChar with {string}")]
fn when_insert_char(world: &mut BusWorld, ch: String) {
    let ch_str = ch.replace("\\n", "\n").replace("\\t", "\t");
    let ch = ch_str.chars().next().expect("single char");
    world.submit_and_process(npr::Command::InsertChar {
        payload: npr::chat_input::InsertChar { ch },
    });
}

#[cucumber::when(expr = "I submit DeleteGrapheme")]
fn when_delete_grapheme(world: &mut BusWorld) {
    world.submit_and_process(npr::Command::DeleteGrapheme);
}

#[cucumber::when(expr = "I submit DeleteGraphemeForward")]
fn when_delete_grapheme_forward(world: &mut BusWorld) {
    world.submit_and_process(npr::Command::DeleteGraphemeForward);
}

#[cucumber::when(expr = "I submit SubmitMessage")]
fn when_submit_message(world: &mut BusWorld) {
    world.submit_and_process(npr::Command::SubmitMessage {
        payload: npr::chat_input::SubmitMessage {
            session_id: world.state.active_session.clone(),
            text: String::new(),
        },
    });
}

#[cucumber::when(expr = "I submit Clear")]
fn when_clear(world: &mut BusWorld) {
    world.submit_and_process(npr::Command::Clear);
}

#[cucumber::when(expr = "I submit Interrupt")]
fn when_interrupt(world: &mut BusWorld) {
    world.submit_and_process(npr::Command::Interrupt);
}

#[cucumber::when(expr = "I submit SetMode {word}")]
fn when_set_mode(world: &mut BusWorld, mode: String) {
    world.submit_and_process(npr::Command::SetMode {
        payload: npr::system::SetMode {
            mode: parse_mode(&mode),
        },
    });
}

#[cucumber::when(expr = "I submit MoveCursorLeft")]
fn when_move_cursor_left(world: &mut BusWorld) {
    world.submit_and_process(npr::Command::MoveCursorLeft);
}

#[cucumber::when(expr = "I submit MoveCursorRight")]
fn when_move_cursor_right(world: &mut BusWorld) {
    world.submit_and_process(npr::Command::MoveCursorRight);
}

#[cucumber::when(expr = "I submit MoveCursorToStart")]
fn when_move_cursor_to_start(world: &mut BusWorld) {
    world.submit_and_process(npr::Command::MoveCursorToStart);
}

#[cucumber::when(expr = "I submit MoveCursorToEnd")]
fn when_move_cursor_to_end(world: &mut BusWorld) {
    world.submit_and_process(npr::Command::MoveCursorToEnd);
}

#[cucumber::when(expr = "I submit MoveCursorWordLeft")]
fn when_move_cursor_word_left(world: &mut BusWorld) {
    world.submit_and_process(npr::Command::MoveCursorWordLeft);
}

#[cucumber::when(expr = "I submit MoveCursorWordRight")]
fn when_move_cursor_word_right(world: &mut BusWorld) {
    world.submit_and_process(npr::Command::MoveCursorWordRight);
}

#[cucumber::when(expr = "I submit MoveCursorUp")]
fn when_move_cursor_up(world: &mut BusWorld) {
    world.submit_and_process(npr::Command::MoveCursorUp);
}

#[cucumber::when(expr = "I submit MoveCursorDown")]
fn when_move_cursor_down(world: &mut BusWorld) {
    world.submit_and_process(npr::Command::MoveCursorDown);
}

#[cucumber::when(expr = "I submit EnqueueUserMessage with text {string}")]
fn when_enqueue_user_message(world: &mut BusWorld, text: String) {
    world.submit_and_process(npr::Command::EnqueueUserMessage {
        payload: npr::chat_input::EnqueueUserMessage {
            session_id: world.state.active_session.clone(),
            text,
        },
    });
}

#[cucumber::when(expr = "I submit CancelStream")]
fn when_cancel_stream(world: &mut BusWorld) {
    world.submit_and_process(npr::Command::CancelStream {
        payload: npr::provider::CancelStream {
            session_id: world.state.active_session.clone(),
        },
    });
}

#[cucumber::when(expr = "I submit SetChatInputText with {string}")]
fn when_set_chat_input_text(world: &mut BusWorld, text: String) {
    world.submit_and_process(npr::Command::SetChatInputText {
        payload: npr::chat_input::SetChatInputText {
            session_id: world.state.active_session.clone(),
            text,
        },
    });
}

#[cucumber::when(expr = "I submit StreamCompleted with reason Finished")]
fn when_stream_completed_finished(world: &mut BusWorld) {
    world.submit_event_and_process(npr::Event::StreamCompleted {
        payload: npr::provider::StreamCompleted {
            session_id: world.state.active_session.clone(),
            reason: npr::provider::StreamCompletedReason::Finished,
            assistant_content: None,
            tool_calls: None,
        },
    });
}

#[cucumber::when(expr = "I submit StreamCompleted with reason Canceled")]
fn when_stream_completed_canceled(world: &mut BusWorld) {
    world.submit_event_and_process(npr::Event::StreamCompleted {
        payload: npr::provider::StreamCompleted {
            session_id: world.state.active_session.clone(),
            reason: npr::provider::StreamCompletedReason::Canceled,
            assistant_content: None,
            tool_calls: None,
        },
    });
}

#[cucumber::when(expr = "I submit PickerInsertChar with {string}")]
fn when_picker_insert_char(world: &mut BusWorld, ch: String) {
    let ch = ch.chars().next().expect("single char");
    world.submit_and_process(npr::Command::PickerInsertChar {
        payload: npr::provider_picker::PickerInsertChar { ch },
    });
}

#[cucumber::when(expr = "I submit PickerBackspace")]
fn when_picker_backspace(world: &mut BusWorld) {
    world.submit_and_process(npr::Command::PickerBackspace);
}

#[cucumber::when(expr = "I submit PickerMoveUp")]
fn when_picker_move_up(world: &mut BusWorld) {
    world.submit_and_process(npr::Command::PickerMoveUp);
}

#[cucumber::when(expr = "I submit PickerMoveDown")]
fn when_picker_move_down(world: &mut BusWorld) {
    world.submit_and_process(npr::Command::PickerMoveDown);
}

#[cucumber::when(expr = "I submit PickerConfirm")]
fn when_picker_confirm(world: &mut BusWorld) {
    world.submit_and_process(npr::Command::PickerConfirm);
}

#[cucumber::when(expr = "I submit PickerMoveCursorLeft")]
fn when_picker_move_cursor_left(world: &mut BusWorld) {
    world.submit_and_process(npr::Command::PickerMoveCursorLeft);
}

#[cucumber::when(expr = "I submit PickerMoveCursorRight")]
fn when_picker_move_cursor_right(world: &mut BusWorld) {
    world.submit_and_process(npr::Command::PickerMoveCursorRight);
}

// --- Then steps ---

#[cucumber::then(expr = "the input buffer should be {string}")]
fn then_input_buffer_should_be(world: &mut BusWorld, expected: String) {
    let actual = world.state.active_chat_input().text().to_owned();
    let expected = expected.replace("\\n", "\n").replace("\\t", "\t");
    assert_eq!(actual, expected, "input buffer mismatch");
}

#[cucumber::then(expr = "the input buffer should be empty")]
fn then_input_buffer_empty(world: &mut BusWorld) {
    let text = world.state.active_chat_input().text().to_owned();
    assert!(text.is_empty(), "expected empty input buffer, got: {text:?}");
}

#[cucumber::then(expr = "the cursor position should be {int}")]
fn then_cursor_position(world: &mut BusWorld, expected: u64) {
    let actual = world.state.active_chat_input().cursor_pos();
    assert_eq!(actual, expected as usize, "cursor position mismatch");
}

#[cucumber::then(expr = "the cursor row should be {int} and column should be {int}")]
fn then_cursor_row_col(world: &mut BusWorld, row: u64, col: u64) {
    let (actual_row, actual_col) = world.state.active_chat_input().cursor_row_col();
    assert_eq!((actual_row, actual_col), (row as usize, col as usize), "cursor row/col mismatch");
}

#[cucumber::then(expr = "the mode should be {word}")]
fn then_mode_should_be(world: &mut BusWorld, mode: String) {
    let expected = parse_mode(&mode);
    assert_eq!(world.state.mode, expected, "mode mismatch");
}

#[cucumber::then(expr = "the chat history should contain {int} entry")]
fn then_chat_history_count(world: &mut BusWorld, count: u64) {
    let actual = world.state.active_session().history().len();
    assert_eq!(actual, count as usize, "expected {count} history entries, got {actual}");
}

#[cucumber::then(expr = "chat history entry {int} should be a User message with text {string}")]
fn then_history_entry_is_user(world: &mut BusWorld, index: u64, text: String) {
    let entry = &world.state.active_session().history()[(index - 1) as usize];
    assert_eq!(
        entry.kind,
        npr::ChatEntryKind::User(text),
        "entry {index} mismatch"
    );
}

#[cucumber::then(expr = "the app should quit")]
fn then_app_should_quit(world: &mut BusWorld) {
    assert!(world.state.should_quit, "expected app to quit");
}

#[cucumber::then(expr = "a SendToLlmProvider command should have been submitted")]
fn then_send_to_llm_submitted(world: &mut BusWorld) {
    let found = world.has_processed_command(|c| matches!(c, npr::Command::SendToLlmProvider { .. }));
    assert!(found, "expected SendToLlmProvider command");
}

#[cucumber::then(expr = "an AssemblePrompt command should have been submitted")]
fn then_assemble_prompt_submitted(world: &mut BusWorld) {
    let found = world.has_processed_command(|c| matches!(c, npr::Command::AssemblePrompt { .. }));
    assert!(found, "expected AssemblePrompt command");
}

#[cucumber::then(expr = "exactly {int} AssemblePrompt command should have been submitted")]
fn then_exactly_n_assemble_prompt(world: &mut BusWorld, count: u64) {
    let n = world.count_processed_commands(|c| matches!(c, npr::Command::AssemblePrompt { .. }));
    assert_eq!(n, count as usize, "expected {count} AssemblePrompt commands, got {n}");
}

#[cucumber::then(expr = "no SendToLlmProvider command should have been submitted")]
fn then_no_send_to_llm_submitted(world: &mut BusWorld) {
    let found = world.has_processed_command(|c| matches!(c, npr::Command::SendToLlmProvider { .. }));
    assert!(!found, "did not expect SendToLlmProvider command");
}

#[cucumber::then(expr = "a CancelStream command should have been submitted")]
fn then_cancel_stream_submitted(world: &mut BusWorld) {
    let found = world.has_processed_command(|c| matches!(c, npr::Command::CancelStream { .. }));
    assert!(found, "expected CancelStream command");
}

#[cucumber::then(expr = "no CancelStream command should have been submitted")]
fn then_no_cancel_stream_submitted(world: &mut BusWorld) {
    let found = world.has_processed_command(|c| matches!(c, npr::Command::CancelStream { .. }));
    assert!(!found, "did not expect CancelStream command");
}

#[cucumber::then(expr = "no commands should be pending")]
fn then_no_pending_commands(world: &mut BusWorld) {
    assert!(!world.bus.has_pending(), "expected no pending commands/events");
}

#[cucumber::then(expr = "the session should be sending")]
fn then_session_sending(world: &mut BusWorld) {
    assert!(world.state.active_session().is_sending(), "expected session to be sending");
}

#[cucumber::then(expr = "the session should be assembling")]
fn then_session_assembling(world: &mut BusWorld) {
    assert!(world.state.active_session().is_assembling(), "expected session to be assembling");
}

#[cucumber::then(expr = "the session should be idle")]
fn then_session_idle(world: &mut BusWorld) {
    assert!(world.state.active_session().is_idle(), "expected session to be idle");
}

#[cucumber::then(expr = "the session should not be streaming")]
fn then_session_not_streaming(world: &mut BusWorld) {
    assert!(!world.state.active_session().is_streaming(), "expected session to not be streaming");
}

#[cucumber::then(expr = "the message queue should be empty")]
fn then_queue_empty(world: &mut BusWorld) {
    assert_eq!(world.state.active_session().queue_len(), 0, "expected empty queue");
}

#[cucumber::then(expr = "the message queue should contain {int} message")]
fn then_queue_count(world: &mut BusWorld, count: u64) {
    assert_eq!(
        world.state.active_session().queue_len(),
        count as usize,
        "queue length mismatch"
    );
}

#[cucumber::then(expr = "message queue entry {int} should be {string}")]
fn then_queue_entry(world: &mut BusWorld, index: u64, text: String) {
    let queue = world.state.active_session().queue();
    assert_eq!(
        queue[(index - 1) as usize], text,
        "queue entry {index} mismatch"
    );
}

#[cucumber::then(expr = "a SetChatInputText command should have been submitted with text {string}")]
fn then_set_chat_input_text_submitted(world: &mut BusWorld, text: String) {
    let text = text.replace("\\n", "\n").replace("\\t", "\t");
    let commands = world.bus.drain_processed_commands();
    let found = commands.iter().any(|c| match &c.command {
        npr::Command::SetChatInputText { payload } => payload.text == text,
        _ => false,
    });
    assert!(found, "expected SetChatInputText command with text {text:?}");
}

#[cucumber::then(expr = "exactly {int} SendToLlmProvider command should have been submitted")]
fn then_exactly_n_send_to_llm(world: &mut BusWorld, count: u64) {
    let n = world.count_processed_commands(|c| matches!(c, npr::Command::SendToLlmProvider { .. }));
    assert_eq!(n, count as usize, "expected {count} SendToLlmProvider commands, got {n}");
}

#[cucumber::then(expr = "the picker filter should be {string}")]
fn then_picker_filter_should_be(world: &mut BusWorld, expected: String) {
    assert_eq!(world.state.picker.filter, expected, "picker filter mismatch");
}

#[cucumber::then(expr = "the picker selection should be {int}")]
fn then_picker_selection_should_be(world: &mut BusWorld, expected: u64) {
    assert_eq!(
        world.state.picker.selection, expected as usize,
        "picker selection mismatch"
    );
}

#[cucumber::then(expr = "the picker cursor position should be {int}")]
fn then_picker_cursor_pos(world: &mut BusWorld, expected: u64) {
    assert_eq!(
        world.state.picker.cursor_pos(), expected as usize,
        "picker cursor position mismatch"
    );
}

#[cucumber::then(expr = "the active provider should be {string}")]
fn then_active_provider_is(world: &mut BusWorld, expected: String) {
    assert_eq!(world.state.active_provider, expected, "active provider mismatch");
}

// ---------------------------------------------------------------------------
// Step definitions — Provider Switch (Phase 1)
// ---------------------------------------------------------------------------

#[cucumber::when(expr = "I submit ProviderSwitch with provider {string}")]
fn when_provider_switch(world: &mut BusWorld, provider_id: String) {
    world.bus.submit_command(npr::Command::ProviderSwitch {
        payload: npr::provider::ProviderSwitch { provider_id },
    });
    world.bus.process_commands(&mut world.state, &world.services);
    world.bus.process_events(&mut world.state, &world.services);
    world.bus.process_commands(&mut world.state, &world.services);
}

#[cucumber::then(expr = "a ProviderSwitched event should have been submitted")]
fn then_provider_switched_event(world: &mut BusWorld) {
    let found = world
        .bus
        .drain_processed_events()
        .iter()
        .any(|e| matches!(e.event, npr::Event::ProviderSwitched { .. }));
    assert!(found, "expected ProviderSwitched event");
}

#[cucumber::then(expr = "chat history entry {int} should be a System message")]
fn then_history_entry_is_system(world: &mut BusWorld, index: u64) {
    let entry = &world.state.active_session().history()[(index - 1) as usize];
    assert!(
        matches!(&entry.kind, npr::ChatEntryKind::System(_)),
        "entry {index} is not a System message"
    );
}

// ---------------------------------------------------------------------------
// Step definitions — Model Refresh (Phase 2)
// ---------------------------------------------------------------------------

#[cucumber::when(expr = "I submit RefreshModels")]
fn when_refresh_models(world: &mut BusWorld) {
    world.submit_and_process(npr::Command::RefreshModels);
}

#[cucumber::given(expr = "a model cache file exists for provider {string} with models {string}")]
fn given_model_cache_file(_world: &mut BusWorld, provider: String, models: String) {
    let mut cache = nullslop_providers::ModelCache::new();
    let model_list: Vec<String> = models.split(", ").map(|s| s.to_owned()).collect();
    cache.entries.insert(provider, model_list);
    let path = nullslop_providers::cache_path();
    let _ = std::fs::remove_file(&path); // Clean up from previous runs.
    cache.save(&path).expect("save cache");
}

#[cucumber::when(expr = "I submit a ModelsRefreshed event for provider {string} with models {string}")]
fn when_models_refreshed_for_provider(world: &mut BusWorld, provider: String, models: String) {
    let model_list: Vec<String> = models.split(", ").map(|s| s.to_owned()).collect();
    let results = std::collections::HashMap::from([(provider.clone(), model_list)]);
    world.submit_event_and_process(npr::Event::ModelsRefreshed {
        payload: npr::provider::ModelsRefreshed {
            results,
            errors: std::collections::HashMap::new(),
        },
    });
}

#[cucumber::when(expr = "I submit a ModelsRefreshed event with 2 providers and 3 models")]
fn when_models_refreshed_2p3m(world: &mut BusWorld) {
    let results = std::collections::HashMap::from([
        ("ollama".to_owned(), vec!["llama3".to_owned()]),
        ("openrouter".to_owned(), vec!["gpt-4".to_owned(), "claude".to_owned()]),
    ]);
    world.submit_event_and_process(npr::Event::ModelsRefreshed {
        payload: npr::provider::ModelsRefreshed {
            results,
            errors: std::collections::HashMap::new(),
        },
    });
}

#[cucumber::when(expr = "I submit a ModelsRefreshed event with 1 provider and 1 model and errors")]
fn when_models_refreshed_with_errors(world: &mut BusWorld) {
    let results = std::collections::HashMap::from([("ollama".to_owned(), vec!["llama3".to_owned()])]);
    let errors = std::collections::HashMap::from([(
        "lmstudio".to_owned(),
        "connection refused".to_owned(),
    )]);
    world.submit_event_and_process(npr::Event::ModelsRefreshed {
        payload: npr::provider::ModelsRefreshed { results, errors },
    });
}

#[cucumber::when(expr = "I submit a ModelsRefreshed event with no results or errors")]
fn when_models_refreshed_empty(world: &mut BusWorld) {
    world.submit_event_and_process(npr::Event::ModelsRefreshed {
        payload: npr::provider::ModelsRefreshed {
            results: std::collections::HashMap::new(),
            errors: std::collections::HashMap::new(),
        },
    });
}

#[cucumber::then(expr = "the model cache should contain {int} provider")]
fn then_model_cache_provider_count(world: &mut BusWorld, count: u64) {
    let cache = world
        .state
        .model_cache
        .as_ref()
        .expect("model cache should be loaded");
    assert_eq!(cache.entries.len(), count as usize, "model cache provider count mismatch");
}

#[cucumber::then(expr = "the model cache entry for {string} should have {int} models")]
fn then_model_cache_entry_models(world: &mut BusWorld, provider: String, count: u64) {
    let cache = world
        .state
        .model_cache
        .as_ref()
        .expect("model cache should be loaded");
    assert_eq!(
        cache.entries[&provider].len(),
        count as usize,
        "model cache entry model count mismatch"
    );
}

#[cucumber::then(expr = "chat history entry {int} should be a System message with text {string}")]
fn then_history_entry_is_system_with_text(world: &mut BusWorld, index: u64, text: String) {
    let entry = &world.state.active_session().history()[(index - 1) as usize];
    assert_eq!(
        entry.kind,
        npr::ChatEntryKind::System(text),
        "entry {index} mismatch"
    );
}

// ---------------------------------------------------------------------------
// Step definitions — Chat Log (Phase 2)
// ---------------------------------------------------------------------------

#[cucumber::when(expr = "I submit PushChatEntry with a User message {string}")]
fn when_push_user_entry(world: &mut BusWorld, text: String) {
    world.bus.submit_command(npr::Command::PushChatEntry {
        payload: npr::chat_input::PushChatEntry {
            session_id: world.state.active_session.clone(),
            entry: npr::ChatEntry::user(text),
        },
    });
    world.bus.process_commands(&mut world.state, &world.services);
    world.bus.process_events(&mut world.state, &world.services);
    world.bus.process_commands(&mut world.state, &world.services);
}

#[cucumber::when(expr = "I submit PushChatEntry with an Actor message from {string} with text {string}")]
fn when_push_actor_entry(world: &mut BusWorld, source: String, text: String) {
    world.bus.submit_command(npr::Command::PushChatEntry {
        payload: npr::chat_input::PushChatEntry {
            session_id: world.state.active_session.clone(),
            entry: npr::ChatEntry::actor(source, text),
        },
    });
    world.bus.process_commands(&mut world.state, &world.services);
    world.bus.process_events(&mut world.state, &world.services);
    world.bus.process_commands(&mut world.state, &world.services);
}

#[cucumber::when(expr = "I submit ScrollUp")]
fn when_scroll_up(world: &mut BusWorld) {
    world.submit_and_process(npr::Command::ScrollUp);
}

#[cucumber::when(expr = "I submit ScrollDown")]
fn when_scroll_down(world: &mut BusWorld) {
    world.submit_and_process(npr::Command::ScrollDown);
}

#[cucumber::then(expr = "a ChatEntrySubmitted event should have been submitted")]
fn then_chat_entry_submitted_event(world: &mut BusWorld) {
    let found = world
        .bus
        .drain_processed_events()
        .iter()
        .any(|e| matches!(e.event, npr::Event::ChatEntrySubmitted { .. }));
    assert!(found, "expected ChatEntrySubmitted event");
}

#[cucumber::then(expr = "chat history entry {int} should be an Actor message from {string} with text {string}")]
fn then_history_entry_is_actor(world: &mut BusWorld, index: u64, source: String, text: String) {
    let entry = &world.state.active_session().history()[(index - 1) as usize];
    assert_eq!(
        entry.kind,
        npr::ChatEntryKind::Actor { source, text },
        "entry {index} mismatch"
    );
}

#[cucumber::then(expr = "the scroll offset should be {int}")]
fn then_scroll_offset(world: &mut BusWorld, expected: u64) {
    assert_eq!(
        world.state.active_session().scroll_offset(),
        expected as u16,
        "scroll offset mismatch"
    );
}

#[cucumber::given(expr = "the session has {int} history entries")]
fn given_session_has_n_entries(world: &mut BusWorld, count: u64) {
    for i in 0..count {
        world
            .state
            .active_session_mut()
            .push_entry(npr::ChatEntry::user(format!("msg {i}")));
    }
}

#[cucumber::given(expr = "the scroll offset is at the top")]
fn given_scroll_offset_at_top(world: &mut BusWorld) {
    world.state.active_session_mut().scroll_up(u16::MAX);
}

// ---------------------------------------------------------------------------
// Step definitions — Shutdown Tracker (Phase 3)
// ---------------------------------------------------------------------------

#[cucumber::given(expr = "shutdown is active")]
fn given_shutdown_active(world: &mut BusWorld) {
    world.state.shutdown_tracker.begin_shutdown();
}

#[cucumber::given(expr = "actor {string} is tracked for shutdown")]
fn given_actor_tracked(world: &mut BusWorld, name: String) {
    world.state.shutdown_tracker.track(&name);
}

#[cucumber::when(expr = "I submit ActorStarting with name {string}")]
fn when_actor_starting(world: &mut BusWorld, name: String) {
    world.submit_event_and_process(npr::Event::ActorStarting {
        payload: npr::actor::ActorStarting { name },
    });
}

#[cucumber::when(expr = "I submit ActorShutdownCompleted with name {string}")]
fn when_actor_shutdown_completed(world: &mut BusWorld, name: String) {
    world.submit_event_and_process(npr::Event::ActorShutdownCompleted {
        payload: npr::actor::ActorShutdownCompleted { name },
    });
}

#[cucumber::then(expr = "the shutdown tracker should have {int} pending actor")]
fn then_shutdown_pending_count(world: &mut BusWorld, count: u64) {
    assert_eq!(
        world.state.shutdown_tracker.pending_names().len(),
        count as usize,
        "shutdown tracker pending count mismatch"
    );
}

#[cucumber::then(expr = "the shutdown tracker pending actors should include {string}")]
fn then_shutdown_pending_includes(world: &mut BusWorld, name: String) {
    assert!(
        world.state.shutdown_tracker.pending_names().contains(&name),
        "expected '{name}' in pending actors"
    );
}

#[cucumber::then(expr = "a ProceedWithShutdown command should have been submitted")]
fn then_proceed_with_shutdown_submitted(world: &mut BusWorld) {
    let found = world.has_processed_command(|c| matches!(c, npr::Command::ProceedWithShutdown { .. }));
    assert!(found, "expected ProceedWithShutdown command");
}

#[cucumber::then(expr = "no ProceedWithShutdown command should have been submitted")]
fn then_no_proceed_with_shutdown_submitted(world: &mut BusWorld) {
    let found = world.has_processed_command(|c| matches!(c, npr::Command::ProceedWithShutdown { .. }));
    assert!(!found, "did not expect ProceedWithShutdown command");
}

// --- Helpers ---

fn parse_mode(name: &str) -> npr::Mode {
    match name.to_lowercase().as_str() {
        "normal" => npr::Mode::Normal,
        "input" => npr::Mode::Input,
        "picker" => npr::Mode::Picker,
        _ => panic!("unknown mode: {name}"),
    }
}

// ---------------------------------------------------------------------------
// Step definitions — Strategy Switching (Phase 3)
// ---------------------------------------------------------------------------

#[cucumber::when(expr = "I submit SwitchPromptStrategy with strategy {string}")]
fn when_switch_prompt_strategy(world: &mut BusWorld, strategy: String) {
    let session_id = world.state.active_session.clone();
    let strategy_id = match strategy.as_str() {
        "passthrough" => npr::PromptStrategyId::passthrough(),
        "sliding_window" => npr::PromptStrategyId::sliding_window(),
        _ => npr::PromptStrategyId::new(strategy),
    };
    world.submit_and_process(npr::Command::SwitchPromptStrategy {
        payload: npr::SwitchPromptStrategy {
            session_id,
            strategy_id,
        },
    });
}

#[cucumber::when(expr = "I submit a PromptStrategySwitched event with strategy {string}")]
fn when_submit_strategy_switched_event(world: &mut BusWorld, strategy: String) {
    let session_id = world.state.active_session.clone();
    let strategy_id = match strategy.as_str() {
        "passthrough" => npr::PromptStrategyId::passthrough(),
        "sliding_window" => npr::PromptStrategyId::sliding_window(),
        _ => npr::PromptStrategyId::new(strategy),
    };
    world.submit_event_and_process(npr::Event::PromptStrategySwitched {
        payload: npr::PromptStrategySwitched {
            session_id,
            strategy_id,
        },
    });
}

#[cucumber::then(expr = "the active strategy should be {string}")]
fn then_active_strategy_is(world: &mut BusWorld, expected: String) {
    let strategy = world.state.active_session().active_strategy();
    assert_eq!(strategy.to_string(), expected, "active strategy mismatch");
}

#[cucumber::then(expr = "a PromptStrategySwitched event should have been submitted")]
fn then_prompt_strategy_switched_event(world: &mut BusWorld) {
    let found = world
        .bus
        .drain_processed_events()
        .iter()
        .any(|e| matches!(e.event, npr::Event::PromptStrategySwitched { .. }));
    assert!(found, "expected PromptStrategySwitched event");
}

#[cucumber::then(expr = "no PromptStrategySwitched event should have been submitted")]
fn then_no_prompt_strategy_switched_event(world: &mut BusWorld) {
    let found = world
        .bus
        .drain_processed_events()
        .iter()
        .any(|e| matches!(e.event, npr::Event::PromptStrategySwitched { .. }));
    assert!(!found, "did not expect PromptStrategySwitched event");
}
