//! Main application state and per-frame rendering.

use derive_more::Debug;
use nullslop_component::AppUiRegistry;
use nullslop_core::{AppCore, AppMsg};
use nullslop_protocol::{Command, Mode};
use ratatui::Frame;
use ratatui_tabs::TabManager;
use ratatui_which_key::WhichKeyState;

use crate::keymap;
use crate::msg::Msg;
use crate::render;
use crate::scope::Scope;
use crate::suspend::{Suspend, SuspendAction};
use crate::{AppStatus, MsgHandler};

/// Type alias for the which-key state parameterized for nullslop.
pub type WhichKeyInstance =
    WhichKeyState<nullslop_protocol::KeyEvent, Scope, Command, crate::keymap::KeyCategory>;

/// Top-level application state and event loop.
#[derive(Debug)]
pub struct TuiApp {
    /// Application core (bus, state, message channel).
    pub core: AppCore,
    /// UI element registry.
    pub ui_registry: AppUiRegistry,
    /// Message channel for the event loop.
    pub events: MsgHandler,
    /// Which-key keybinding system state.
    #[debug(skip)]
    pub which_key: WhichKeyInstance,
    /// Deferred suspend action queue (e.g., for external editor).
    pub(crate) suspend: Suspend,
    /// Background event stream. Set by [`run`](crate::run::run).
    #[debug(skip)]
    pub event_task: Option<tokio::task::JoinHandle<()>>,
    /// Runtime services.
    pub services: nullslop_services::Services,
    /// Current application lifecycle status.
    pub status: AppStatus,
    /// Tab manager for rendering the tab bar.
    pub tab_manager: TabManager,
}

impl TuiApp {
    /// Creates a new application with the given services.
    #[must_use]
    pub fn new(services: nullslop_services::Services) -> Self {
        let mut core = AppCore::new();
        let mut ui_registry = AppUiRegistry::new();
        nullslop_component::register_all(&mut core.bus, &mut ui_registry);
        let keymap = keymap::init();
        let which_key = WhichKeyInstance::new(keymap, Scope::Normal);

        Self {
            core,
            ui_registry,
            events: MsgHandler::new(),
            which_key,
            suspend: Suspend::new(),
            event_task: None,
            services,
            status: AppStatus::Starting,
            tab_manager: crate::render::init_tab_manager(),
        }
    }

    /// Creates a new application with pre-built core and services.
    ///
    /// Use this when the caller has already registered components
    /// and set up the actor host on the core.
    #[must_use]
    pub fn new_with_core(
        services: nullslop_services::Services,
        core: nullslop_core::AppCore,
    ) -> Self {
        let mut ui_registry = AppUiRegistry::new();
        nullslop_component::register_tui_elements(&mut ui_registry);
        let keymap = keymap::init();
        let which_key = WhichKeyInstance::new(keymap, Scope::Normal);

        Self {
            core,
            ui_registry,
            events: MsgHandler::new(),
            which_key,
            suspend: Suspend::new(),
            event_task: None,
            services,
            status: AppStatus::Starting,
            tab_manager: crate::render::init_tab_manager(),
        }
    }

    /// Processes a single message.
    pub fn handle_msg(&mut self, msg: Msg) {
        match msg {
            Msg::Tick => {}
            Msg::Input(event) => {
                if let crossterm::event::Event::Key(key) = event
                    && key.kind == crossterm::event::KeyEventKind::Press
                    && let Some(protocol_key) = crate::convert::from_crossterm(key)
                    && let Some(cmd) = self.which_key.handle_key(protocol_key)
                {
                    self.route_command(cmd);
                }
            }
            Msg::Command(cmd) => {
                self.route_command(cmd);
            }
        }
    }

    /// Routes a command to the appropriate handler.
    ///
    /// Commands that need `TuiApp`-level state (which-key toggle, editor suspend)
    /// are handled directly. All other commands go through the core channel.
    fn route_command(&mut self, cmd: Command) {
        match cmd {
            Command::ToggleWhichKey => {
                self.which_key.toggle();
            }
            Command::EditInput => {
                let initial_content = self.core.state.read().chat_input.text().to_string();
                self.suspend.request(SuspendAction::Edit {
                    initial_content,
                    on_result: Box::new(|result| result),
                });
            }
            _ => {
                let _ = self.core.sender().send(AppMsg::Command {
                    command: cmd,
                    source: None,
                });
            }
        }
    }

    /// Renders the application for a single frame.
    pub fn render(&mut self, frame: &mut Frame<'_>) {
        render::render(self, frame);
    }
}

/// Returns the scope corresponding to the given mode.
pub fn scope_for_mode(mode: Mode) -> Scope {
    match mode {
        Mode::Normal => Scope::Normal,
        Mode::Input => Scope::Input,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use nullslop_actor_host::FakeActorHost;
    use nullslop_protocol as npr;

    use super::*;

    fn key_event(code: KeyCode) -> crossterm::event::Event {
        crossterm::event::Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn create_test_app() -> TuiApp {
        // Leaking the runtime is acceptable for tests — each test gets its own.
        let rt = Box::leak(Box::new(
            tokio::runtime::Runtime::new().expect("test runtime"),
        ));
        let handle = rt.handle().clone();
        let actor_host: Arc<dyn nullslop_actor_host::ActorHost> = Arc::new(FakeActorHost::new());
        let services = nullslop_services::Services::new(handle, actor_host);
        TuiApp::new(services)
    }

    #[test]
    fn app_new_starts_in_normal_scope() {
        // Given a new App.
        let app = create_test_app();

        // When inspecting the which_key scope.
        assert_eq!(*app.which_key.scope(), Scope::Normal);

        // Then it starts in Normal scope.
    }

    #[test]
    fn app_normal_enter_enters_input() {
        // Given an App in Normal scope.
        let mut app = create_test_app();
        assert_eq!(*app.which_key.scope(), Scope::Normal);

        // When pressing 'i'.
        app.handle_msg(Msg::Input(key_event(KeyCode::Char('i'))));

        // Then process core and verify state.
        app.core.tick();
        assert_eq!(app.core.state.read().mode, Mode::Input);
    }

    #[test]
    fn app_normal_esc_quits() {
        // Given an App in Normal scope.
        let mut app = create_test_app();

        // When pressing 'q'.
        app.handle_msg(Msg::Input(key_event(KeyCode::Char('q'))));

        // Then process core and verify.
        let should_quit = app.core.tick().should_quit;
        assert!(should_quit);
    }

    #[test]
    fn app_input_enter_submits() {
        // Given an App in Input scope with "hello" in buffer.
        let mut app = create_test_app();
        app.which_key.set_scope(Scope::Input);
        app.core
            .state
            .write()
            .chat_input
            .replace_all("hello".to_string());

        // When pressing Enter.
        app.handle_msg(Msg::Input(key_event(KeyCode::Enter)));

        // Then process core and verify.
        app.core.tick();
        let guard = app.core.state.read();
        assert_eq!(guard.chat_history.len(), 1);
        assert_eq!(
            guard.chat_history[0].kind,
            npr::ChatEntryKind::User("hello".to_string())
        );
        assert!(guard.chat_input.is_empty());
    }

    #[test]
    fn app_input_esc_back_to_normal() {
        // Given an App in Input scope.
        let mut app = create_test_app();
        app.which_key.set_scope(Scope::Input);

        // When pressing Esc.
        app.handle_msg(Msg::Input(key_event(KeyCode::Esc)));

        // Then process core and verify.
        app.core.tick();
        assert_eq!(app.core.state.read().mode, Mode::Normal);
    }

    #[test]
    fn app_toggle_which_key_handled_directly() {
        // Given an App with inactive which_key.
        let mut app = create_test_app();
        assert!(!app.which_key.active);

        // When routing ToggleWhichKey directly.
        app.route_command(Command::ToggleWhichKey);

        // Then which_key is active (handled directly, not through core).
        assert!(app.which_key.active);
    }

    #[test]
    fn app_push_chat_entry_routes_through_bus() {
        // Given an App with components registered.
        let mut app = create_test_app();

        // When routing a PushChatEntry with an actor entry.
        app.route_command(Command::PushChatEntry {
            payload: npr::chat_input::PushChatEntry {
                entry: npr::ChatEntry::actor("nullslop-echo", "HELLO"),
            },
        });

        // Then process core and verify.
        app.core.tick();
        let guard = app.core.state.read();
        assert_eq!(guard.chat_history.len(), 1);
        assert_eq!(
            guard.chat_history[0].kind,
            npr::ChatEntryKind::Actor {
                source: "nullslop-echo".to_string(),
                text: "HELLO".to_string(),
            }
        );
    }

    #[test]
    fn scope_for_mode_maps_correctly() {
        // Given all Mode variants.
        // When mapping each mode to a scope.
        // Then each mode maps to its corresponding scope.
        assert_eq!(scope_for_mode(Mode::Normal), Scope::Normal);
        assert_eq!(scope_for_mode(Mode::Input), Scope::Input);
    }
}
