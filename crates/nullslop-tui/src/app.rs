//! Main application state and per-frame rendering.

use derive_more::Debug;
use nullslop_core::{AppCore, AppMsg};
use nullslop_plugin_ui::UiRegistry;
use nullslop_protocol::{Command, Mode};
use ratatui::Frame;
use ratatui_which_key::WhichKeyState;

use crate::keymap;
use crate::msg::Msg;
use crate::render;
use crate::scope::Scope;
use crate::suspend::{Suspend, SuspendAction};
use crate::{AppStatus, MsgHandler};

/// Type alias for the which-key state parameterized for nullslop.
pub type WhichKeyInstance =
    WhichKeyState<nullslop_core::KeyEvent, Scope, Command, crate::keymap::KeyCategory>;

/// Top-level application state and event loop.
#[derive(Debug)]
pub struct TuiApp {
    /// Application core (bus, state, message channel).
    pub core: AppCore,
    /// UI element registry.
    pub ui_registry: UiRegistry,
    /// Message channel for the event loop.
    pub events: MsgHandler,
    /// Which-key keybinding system state.
    #[debug(skip)]
    pub which_key: WhichKeyInstance,
    /// Deferred suspend action queue (e.g., for external editor).
    pub(crate) suspend: Suspend,
    /// Background event stream task handle. Set by [`run`](crate::run::run).
    #[debug(skip)]
    pub event_task: Option<tokio::task::JoinHandle<()>>,
    /// Runtime services (tokio handle, extension host). Set during startup.
    pub services: Option<nullslop_services::Services>,
    /// Current application lifecycle status.
    pub status: AppStatus,
}

impl TuiApp {
    /// Creates a new application with default state.
    #[must_use]
    pub fn new() -> Self {
        let mut core = AppCore::new();
        let mut ui_registry = UiRegistry::new();
        nullslop_plugin::register_all(&mut core.bus, &mut ui_registry);
        let keymap = keymap::init();
        let which_key = WhichKeyInstance::new(keymap, Scope::Normal);

        Self {
            core,
            ui_registry,
            events: MsgHandler::new(),
            which_key,
            suspend: Suspend::new(),
            event_task: None,
            services: None,
            status: AppStatus::Starting,
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
            Msg::ExtensionsReady(registrations) => {
                let _ = self.core.sender().send(AppMsg::ExtensionsReady(registrations));
            }
        }
    }

    /// Routes a command to the appropriate handler.
    ///
    /// Commands that need `TuiApp`-level state (which-key toggle, editor suspend)
    /// are handled directly. All other commands go through the core channel.
    fn route_command(&mut self, cmd: Command) {
        match cmd {
            Command::AppToggleWhichKey => {
                self.which_key.toggle();
            }
            Command::AppEditInput => {
                let initial_content = self.core.state.read().chat_input.input_buffer.clone();
                self.suspend.request(SuspendAction::Edit {
                    initial_content,
                    on_result: Box::new(|result| result),
                });
            }
            _ => {
                let _ = self.core.sender().send(AppMsg::Command(cmd));
            }
        }
    }

    /// Renders the application for a single frame.
    pub fn render(&mut self, frame: &mut Frame<'_>) {
        render::render(self, frame);
    }
}

impl Default for TuiApp {
    fn default() -> Self {
        Self::new()
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
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use nullslop_protocol as npr;

    use super::*;

    fn key_event(code: KeyCode) -> crossterm::event::Event {
        crossterm::event::Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    #[test]
    fn app_new_starts_in_normal_scope() {
        // Given a new App.
        let app = TuiApp::new();

        // Then which_key scope is Normal.
        assert_eq!(*app.which_key.scope(), Scope::Normal);
    }

    #[test]
    fn app_normal_enter_enters_input() {
        // Given an App in Normal scope.
        let mut app = TuiApp::new();
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
        let mut app = TuiApp::new();

        // When pressing 'q'.
        app.handle_msg(Msg::Input(key_event(KeyCode::Char('q'))));

        // Then process core and verify.
        let should_quit = app.core.tick().should_quit;
        assert!(should_quit);
    }

    #[test]
    fn app_input_enter_submits() {
        // Given an App in Input scope with "hello" in buffer.
        let mut app = TuiApp::new();
        app.which_key.set_scope(Scope::Input);
        app.core.state.write().chat_input.input_buffer = "hello".to_string();

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
        assert!(guard.chat_input.input_buffer.is_empty());
    }

    #[test]
    fn app_input_esc_back_to_normal() {
        // Given an App in Input scope.
        let mut app = TuiApp::new();
        app.which_key.set_scope(Scope::Input);

        // When pressing Esc.
        app.handle_msg(Msg::Input(key_event(KeyCode::Esc)));

        // Then process core and verify.
        app.core.tick();
        assert_eq!(app.core.state.read().mode, Mode::Normal);
    }

    #[test]
    fn app_input_char_appends() {
        // Given an App in Input scope.
        let mut app = TuiApp::new();
        app.which_key.set_scope(Scope::Input);

        // When pressing 'x'.
        app.handle_msg(Msg::Input(key_event(KeyCode::Char('x'))));

        // Then process core and verify.
        app.core.tick();
        assert_eq!(app.core.state.read().chat_input.input_buffer, "x");
    }

    #[test]
    fn app_input_backspace_deletes() {
        // Given an App in Input scope with "ab" in buffer.
        let mut app = TuiApp::new();
        app.which_key.set_scope(Scope::Input);
        app.core.state.write().chat_input.input_buffer = "ab".to_string();

        // When pressing Backspace.
        app.handle_msg(Msg::Input(key_event(KeyCode::Backspace)));

        // Then process core and verify.
        app.core.tick();
        assert_eq!(app.core.state.read().chat_input.input_buffer, "a");
    }

    #[test]
    fn app_toggle_which_key_handled_directly() {
        // Given an App with inactive which_key.
        let mut app = TuiApp::new();
        assert!(!app.which_key.active);

        // When routing AppToggleWhichKey directly.
        app.route_command(Command::AppToggleWhichKey);

        // Then which_key is active (handled directly, not through core).
        assert!(app.which_key.active);
    }

    #[test]
    fn app_extension_command_routes_through_bus() {
        // Given an App with plugins registered.
        let mut app = TuiApp::new();

        // When routing a CustomCommand (echo).
        app.route_command(Command::CustomCommand {
            payload: npr::command::CustomCommand {
                name: "echo".to_string(),
                args: serde_json::json!({"text": "hello"}),
            },
        });

        // Then process core and verify.
        app.core.tick();
        let guard = app.core.state.read();
        assert_eq!(guard.chat_history.len(), 1);
        assert_eq!(
            guard.chat_history[0].kind,
            npr::ChatEntryKind::System("hello".to_string())
        );
    }

    #[test]
    fn scope_for_mode_maps_correctly() {
        assert_eq!(scope_for_mode(Mode::Normal), Scope::Normal);
        assert_eq!(scope_for_mode(Mode::Input), Scope::Input);
    }
}
