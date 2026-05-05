//! Cucumber `World` wrapping a full [`TuiApp`] for e2e scenario testing.
//!
//! The [`TuiWorld`] spins up a real application with fake services,
//! enabling keystroke simulation and state assertions through
//! natural-language step definitions.

use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use cucumber::World;
use nullslop_actor_host::FakeActorHost;
use nullslop_tui::{Scope, TuiApp};

/// Cucumber world wrapping a full [`TuiApp`].
///
/// Created fresh for each scenario. Provides methods for simulating
/// keystrokes and asserting application state.
#[derive(Debug, World)]
#[world(init = Self::new_app)]
pub struct TuiWorld {
    /// The full TUI application under test.
    pub app: TuiApp,
}

impl TuiWorld {
    /// Creates a new world with a `TuiApp` backed by fake services.
    fn new_app() -> Self {
        let rt = Box::leak(Box::new(
            tokio::runtime::Runtime::new().expect("test runtime"),
        ));
        let handle = rt.handle().clone();
        let actor_host: Arc<dyn nullslop_actor_host::ActorHost> = Arc::new(FakeActorHost::new());
        let llm = nullslop_services::providers::LlmServiceFactoryService::new(Arc::new(
            nullslop_providers::FakeLlmServiceFactory::new(vec![]),
        ));
        let config = nullslop_providers::ProvidersConfig {
            providers: vec![],
            aliases: vec![],
            default_provider: None,
        };
        let services = nullslop_services::Services::new(
            handle,
            actor_host,
            llm,
            nullslop_providers::ProviderRegistryService::new(
                nullslop_providers::ProviderRegistry::from_config(config).expect("test registry"),
            ),
            nullslop_providers::ApiKeysService::new(nullslop_providers::ApiKeys::new()),
            nullslop_providers::ConfigStorageService::new(Arc::new(
                nullslop_providers::InMemoryConfigStorage::new(),
            )),
        );
        Self {
            app: TuiApp::new(services),
        }
    }

    /// Sends a keystroke to the app and ticks the core.
    fn press_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        let event = crossterm::event::Event::Key(KeyEvent::new(code, modifiers));
        self.app.handle_msg(nullslop_tui::msg::Msg::Input(event));
        self.app.core.tick();
    }

    /// Routes a command through the app's message pipeline and ticks the core.
    fn route_command(&mut self, cmd: nullslop_protocol::Command) {
        self.app.handle_msg(nullslop_tui::msg::Msg::Command(cmd));
        self.app.core.tick();
    }
}

// ---------------------------------------------------------------------------
// Step definitions
// ---------------------------------------------------------------------------

/// Parses a human-readable key name into a [`KeyCode`].
fn parse_key_code(name: &str) -> KeyCode {
    match name.to_lowercase().as_str() {
        "enter" => KeyCode::Enter,
        "esc" | "escape" => KeyCode::Esc,
        "backspace" => KeyCode::Backspace,
        "tab" => KeyCode::Tab,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "delete" => KeyCode::Delete,
        "space" => KeyCode::Char(' '),
        s if s.len() == 1 => KeyCode::Char(s.chars().next().expect("single char")),
        _ => panic!("unknown key: {name}"),
    }
}

/// Parses a human-readable modifier name into [`KeyModifiers`].
fn parse_modifier(name: &str) -> KeyModifiers {
    match name.to_lowercase().as_str() {
        "shift" => KeyModifiers::SHIFT,
        "ctrl" | "control" => KeyModifiers::CONTROL,
        "alt" => KeyModifiers::ALT,
        _ => panic!("unknown modifier: {name}"),
    }
}

/// Parses a human-readable mode name into [`nullslop_protocol::Mode`].
fn parse_mode(name: &str) -> nullslop_protocol::Mode {
    match name.to_lowercase().as_str() {
        "normal" => nullslop_protocol::Mode::Normal,
        "input" => nullslop_protocol::Mode::Input,
        "picker" => nullslop_protocol::Mode::Picker,
        _ => panic!("unknown mode: {name}"),
    }
}

// --- Given steps ---

/// World is already initialised with a fresh TuiApp.
#[cucumber::given(expr = "a new app")]
fn given_a_new_app(_world: &mut TuiWorld) {}

/// Sets the app's which-key scope to match the given mode.
#[cucumber::given(expr = "the app is in {word} mode")]
fn given_app_in_mode(world: &mut TuiWorld, mode: String) {
    let scope = match parse_mode(&mode) {
        nullslop_protocol::Mode::Normal => Scope::Normal,
        nullslop_protocol::Mode::Input => Scope::Input,
        nullslop_protocol::Mode::Picker => Scope::Picker,
    };
    world.app.which_key.set_scope(scope);
}

/// Pre-fills the active chat input buffer with the given text.
#[cucumber::given(expr = "the input buffer contains {string}")]
fn given_input_buffer_contains(world: &mut TuiWorld, text: String) {
    world
        .app
        .core
        .state
        .write()
        .active_chat_input_mut()
        .replace_all(text.to_owned());
}

/// Sets the active provider to a dummy value so message submission works.
#[cucumber::given(expr = "the active provider is set")]
fn given_active_provider_set(world: &mut TuiWorld) {
    world.app.core.state.write().active_provider = "test".to_owned();
}

// --- When steps ---

/// Simulates the user pressing a single key (no modifiers).
#[cucumber::when(expr = "the user presses {word}")]
fn when_user_presses_key(world: &mut TuiWorld, key: String) {
    let code = parse_key_code(&key);
    world.press_key(code, KeyModifiers::NONE);
}

/// Simulates the user pressing a key with a modifier.
#[cucumber::when(expr = "the user presses {word} with {word}")]
fn when_user_presses_key_with_mod(world: &mut TuiWorld, key: String, modifier: String) {
    let code = parse_key_code(&key);
    let mods = parse_modifier(&modifier);
    world.press_key(code, mods);
}

/// Routes a PushChatEntry command with an actor-sourced message.
#[cucumber::when(
    expr = "the app routes the PushChatEntry command with an actor message from {string} with text {string}"
)]
fn when_routes_push_chat_entry(world: &mut TuiWorld, source: String, text: String) {
    world.route_command(nullslop_protocol::Command::PushChatEntry {
        payload: nullslop_protocol::chat_input::PushChatEntry {
            session_id: nullslop_protocol::SessionId::new(),
            entry: nullslop_protocol::ChatEntry::actor(source, text),
        },
    });
}

/// Routes a ToggleWhichKey command directly.
#[cucumber::when(expr = "the app routes the ToggleWhichKey command")]
fn when_routes_toggle_which_key(world: &mut TuiWorld) {
    world.route_command(nullslop_protocol::Command::ToggleWhichKey);
}

// --- Then steps ---

/// Asserts the application's current mode matches the expected value.
#[cucumber::then(expr = "the mode should be {word}")]
fn then_mode_should_be(world: &mut TuiWorld, mode: String) {
    let expected = parse_mode(&mode);
    let actual = world.app.core.state.read().mode;
    assert_eq!(
        actual, expected,
        "expected mode {expected:?}, got {actual:?}"
    );
}

/// Asserts the application has requested to quit.
#[cucumber::then(expr = "the app should quit")]
fn then_app_should_quit(world: &mut TuiWorld) {
    let should_quit = world.app.core.state.read().should_quit;
    assert!(
        should_quit,
        "expected app to quit, but should_quit is false"
    );
}

/// Asserts the active chat input buffer is empty.
#[cucumber::then(expr = "the input buffer should be empty")]
fn then_input_buffer_empty(world: &mut TuiWorld) {
    let text = world
        .app
        .core
        .state
        .read()
        .active_chat_input()
        .text()
        .to_owned();
    assert!(
        text.is_empty(),
        "expected empty input buffer, got: {text:?}"
    );
}

/// Asserts the active chat input buffer matches the expected text.
#[cucumber::then(expr = "the input buffer should be {string}")]
fn then_input_buffer_should_be(world: &mut TuiWorld, expected: String) {
    let actual = world
        .app
        .core
        .state
        .read()
        .active_chat_input()
        .text()
        .to_owned();
    // Cucumber {string} captures literal text, so handle common escape sequences.
    let expected = expected.replace("\\n", "\n").replace("\\t", "\t");
    assert_eq!(actual, expected, "input buffer mismatch");
}

/// Asserts the active session's chat history contains the expected number of entries.
#[cucumber::then(expr = "the chat history should contain {int} entry")]
fn then_chat_history_count(world: &mut TuiWorld, count: u64) {
    let actual = world.app.core.state.read().active_session().history().len();
    assert_eq!(
        actual, count as usize,
        "expected {count} history entries, got {actual}"
    );
}

/// Asserts that a specific history entry is a user message with the given text.
#[cucumber::then(expr = "the chat history entry {int} should be a user message with text {string}")]
fn then_history_entry_is_user(world: &mut TuiWorld, index: u64, text: String) {
    let guard = world.app.core.state.read();
    let entry = &guard.active_session().history()[(index - 1) as usize];
    assert_eq!(
        entry.kind,
        nullslop_protocol::ChatEntryKind::User(text),
        "entry {index} is not a User message with the expected text"
    );
}

/// Asserts that a specific history entry is an actor message from the given source.
#[cucumber::then(
    expr = "the chat history entry {int} should be an actor message from {string} with text {string}"
)]
fn then_history_entry_is_actor(world: &mut TuiWorld, index: u64, source: String, text: String) {
    let guard = world.app.core.state.read();
    let entry = &guard.active_session().history()[(index - 1) as usize];
    assert_eq!(
        entry.kind,
        nullslop_protocol::ChatEntryKind::Actor { source, text },
        "entry {index} is not an Actor message with expected source/text"
    );
}

/// Asserts the which-key popup is active.
#[cucumber::then(expr = "which-key should be active")]
fn then_which_key_active(world: &mut TuiWorld) {
    assert!(
        world.app.which_key.active,
        "expected which-key to be active"
    );
}

/// Asserts the which-key popup is inactive.
#[cucumber::then(expr = "which-key should be inactive")]
fn then_which_key_inactive(world: &mut TuiWorld) {
    assert!(
        !world.app.which_key.active,
        "expected which-key to be inactive"
    );
}

/// Asserts the which-key scope is Normal.
#[cucumber::then(expr = "the which-key scope should be Normal")]
fn then_which_key_scope_normal(world: &mut TuiWorld) {
    assert_eq!(
        *world.app.which_key.scope(),
        Scope::Normal,
        "expected Normal scope"
    );
}

// ---------------------------------------------------------------------------
// Step definitions — Headless Script Execution (Phase 3)
// ---------------------------------------------------------------------------

/// Runs a headless script through the keymap pipeline.
///
/// Parses the script content into key sequences via `parse_key_sequence`,
/// feeds each key through the which-key state machine, and routes resulting
/// commands through the app's message pipeline.
#[cucumber::when(expr = "I run the headless script {string}")]
fn when_run_headless_script(world: &mut TuiWorld, script: String) {
    run_headless_script(world, &script);
}

/// Runs an empty headless script (no keys pressed).
#[cucumber::when(expr = "I run an empty headless script")]
fn when_run_empty_headless_script(world: &mut TuiWorld) {
    run_headless_script(world, "");
}

/// Shared implementation for running a headless script.
fn run_headless_script(world: &mut TuiWorld, content: &str) {
    let leader = nullslop_protocol::KeyEvent {
        key: nullslop_protocol::Key::Char('\\'),
        modifiers: nullslop_protocol::Modifiers::none(),
    };
    let lines: Vec<Vec<nullslop_protocol::KeyEvent>> = content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| ratatui_which_key::parse_key_sequence(line, &leader))
        .collect();

    for keys in lines {
        for key in keys {
            let state_read = world.app.core.state.read();
            let scope = nullslop_tui::app::scope_for_mode(state_read.mode, state_read.active_tab);
            drop(state_read);
            world.app.which_key.set_scope(scope);
            if let Some(cmd) = world.app.which_key.handle_key(key) {
                world.route_command(cmd);
            }
        }
    }
}

/// Asserts the application has NOT requested to quit.
#[cucumber::then(expr = "the app should not quit")]
fn then_app_should_not_quit(world: &mut TuiWorld) {
    let should_quit = world.app.core.state.read().should_quit;
    assert!(
        !should_quit,
        "expected app to not quit, but should_quit is true"
    );
}
