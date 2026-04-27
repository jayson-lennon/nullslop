//! Main application state and per-frame rendering.

use derive_more::Debug;
use nullslop_core::State;
use nullslop_plugin::Bus;
use nullslop_protocol::{Command, Mode};
use ratatui::Frame;
use ratatui_which_key::WhichKeyState;

use crate::keymap;
use crate::msg::Msg;
use crate::render;
use crate::scope::Scope;
use crate::services::Services;
use crate::suspend::{Suspend, SuspendAction};
use crate::{AppStatus, MsgHandler, TuiState};

/// Type alias for the which-key state parameterized for nullslop.
pub type WhichKeyInstance =
    WhichKeyState<crossterm::event::KeyEvent, Scope, Command, crate::keymap::KeyCategory>;

/// Top-level application state and event loop.
#[derive(Debug)]
pub struct TuiApp {
    /// Shared domain state (chat history, extensions).
    pub state: State,
    /// Ephemeral TUI state (scroll offset).
    pub tui_state: TuiState,
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
    pub services: Option<Services>,
    /// Current application lifecycle status.
    pub status: AppStatus,
    /// Plugin command/event bus.
    #[debug(skip)]
    pub bus: Bus,
}

impl TuiApp {
    /// Creates a new application with default state.
    #[must_use]
    pub fn new() -> Self {
        let keymap = keymap::init();
        let which_key = WhichKeyInstance::new(keymap, Scope::Normal);
        let mut bus = Bus::new();
        crate::plugin::register_all(&mut bus);

        Self {
            state: State::new(nullslop_core::AppData::new()),
            tui_state: TuiState::new(),
            events: MsgHandler::new(),
            which_key,
            suspend: Suspend::new(),
            event_task: None,
            services: None,
            status: AppStatus::Starting,
            bus,
        }
    }

    /// Processes a single message.
    pub fn handle_msg(&mut self, msg: Msg) {
        match msg {
            Msg::Tick => {}
            Msg::Input(event) => {
                if let crossterm::event::Event::Key(key) = event
                    && key.kind == crossterm::event::KeyEventKind::Press
                    && let Some(cmd) = self.which_key.handle_key(key)
                {
                    self.route_command(cmd);
                }
            }
            Msg::Command(cmd) => {
                self.route_command(cmd);
            }
            Msg::ExtensionsReady(registrations) => {
                for reg in registrations {
                    self.state.write().extensions_mut().register(reg);
                }
                tracing::info!("extensions ready");
            }
        }
    }

    /// Routes a command to the appropriate handler.
    ///
    /// Commands that need `TuiApp`-level state (which-key toggle, editor suspend)
    /// are handled directly. All other commands go through the bus.
    fn route_command(&mut self, cmd: Command) {
        match cmd {
            Command::AppToggleWhichKey => {
                self.which_key.toggle();
            }
            Command::AppEditInput => {
                let initial_content = self.state.read().input_buffer.clone();
                self.suspend.request(SuspendAction::Edit {
                    initial_content,
                    on_result: Box::new(|result| result),
                });
            }
            _ => {
                self.bus.submit_command(cmd);
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
pub(crate) fn scope_for_mode(mode: Mode) -> Scope {
    match mode {
        Mode::Normal => Scope::Normal,
        Mode::Input => Scope::Input,
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

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

        // When pressing Enter.
        app.handle_msg(Msg::Input(key_event(KeyCode::Enter)));

        // Then the bus processed AppSetMode(Input) — but scope is synced
        // separately in run.rs. The command was submitted to the bus.
        // Process the bus to verify state.
        {
            let mut guard = app.state.write();
            app.bus.process_commands(&mut guard);
        }
        assert_eq!(app.state.read().mode, Mode::Input);
    }

    #[test]
    fn app_normal_esc_quits() {
        // Given an App in Normal scope.
        let mut app = TuiApp::new();

        // When pressing Esc.
        app.handle_msg(Msg::Input(key_event(KeyCode::Esc)));

        // Then AppQuit was submitted to the bus. Process it.
        {
            let mut guard = app.state.write();
            app.bus.process_commands(&mut guard);
        }
        assert!(app.state.read().should_quit);
    }

    #[test]
    fn app_input_enter_submits() {
        // Given an App in Input scope with "hello" in buffer.
        let mut app = TuiApp::new();
        app.which_key.set_scope(Scope::Input);
        app.state.write().input_buffer = "hello".to_string();

        // When pressing Enter.
        app.handle_msg(Msg::Input(key_event(KeyCode::Enter)));

        // Then process bus and verify.
        {
            let mut guard = app.state.write();
            app.bus.process_commands(&mut guard);
        }
        let guard = app.state.read();
        assert_eq!(guard.chat_history.len(), 1);
        assert_eq!(
            guard.chat_history[0].kind,
            nullslop_protocol::ChatEntryKind::User("hello".to_string())
        );
        assert!(guard.input_buffer.is_empty());
    }

    #[test]
    fn app_input_esc_back_to_normal() {
        // Given an App in Input scope.
        let mut app = TuiApp::new();
        app.which_key.set_scope(Scope::Input);

        // When pressing Esc.
        app.handle_msg(Msg::Input(key_event(KeyCode::Esc)));

        // Then AppSetMode(Normal) was submitted to the bus. Process it.
        {
            let mut guard = app.state.write();
            app.bus.process_commands(&mut guard);
        }
        assert_eq!(app.state.read().mode, Mode::Normal);
    }

    #[test]
    fn app_input_char_appends() {
        // Given an App in Input scope.
        let mut app = TuiApp::new();
        app.which_key.set_scope(Scope::Input);

        // When pressing 'x'.
        app.handle_msg(Msg::Input(key_event(KeyCode::Char('x'))));

        // Then process bus and verify.
        {
            let mut guard = app.state.write();
            app.bus.process_commands(&mut guard);
        }
        assert_eq!(app.state.read().input_buffer, "x");
    }

    #[test]
    fn app_input_backspace_deletes() {
        // Given an App in Input scope with "ab" in buffer.
        let mut app = TuiApp::new();
        app.which_key.set_scope(Scope::Input);
        app.state.write().input_buffer = "ab".to_string();

        // When pressing Backspace.
        app.handle_msg(Msg::Input(key_event(KeyCode::Backspace)));

        // Then process bus and verify.
        {
            let mut guard = app.state.write();
            app.bus.process_commands(&mut guard);
        }
        assert_eq!(app.state.read().input_buffer, "a");
    }

    #[test]
    fn app_toggle_which_key_handled_directly() {
        // Given an App with inactive which_key.
        let mut app = TuiApp::new();
        assert!(!app.which_key.active);

        // When routing AppToggleWhichKey directly.
        app.route_command(Command::AppToggleWhichKey);

        // Then which_key is active (handled directly, not through bus).
        assert!(app.which_key.active);
    }

    #[test]
    fn app_extension_command_routes_through_bus() {
        // Given an App with services.
        let mut app = TuiApp::new();

        // When routing a CustomCommand (echo).
        app.route_command(Command::CustomCommand {
            payload: nullslop_protocol::command::CustomCommand {
                name: "echo".to_string(),
                args: serde_json::json!({"text": "hello"}),
            },
        });

        // Then process bus and verify.
        {
            let mut guard = app.state.write();
            app.bus.process_commands(&mut guard);
        }
        let guard = app.state.read();
        assert_eq!(guard.chat_history.len(), 1);
        assert_eq!(
            guard.chat_history[0].kind,
            nullslop_protocol::ChatEntryKind::System("hello".to_string())
        );
    }

    #[test]
    fn scope_for_mode_maps_correctly() {
        assert_eq!(scope_for_mode(Mode::Normal), Scope::Normal);
        assert_eq!(scope_for_mode(Mode::Input), Scope::Input);
    }
}
