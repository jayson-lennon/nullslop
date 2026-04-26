//! Main application state and per-frame rendering.

use derive_more::Debug;
use nullslop_core::State;
use ratatui::Frame;
use ratatui_which_key::WhichKeyState;

use crate::command::{self, TuiCommand};
use crate::keymap;
use crate::msg::Msg;
use crate::render;
use crate::scope::Scope;
use crate::services::Services;
use crate::suspend::Suspend;
use crate::{AppStatus, MsgHandler, TuiState};

/// Type alias for the which-key state parameterized for nullslop.
pub type WhichKeyInstance =
    WhichKeyState<crossterm::event::KeyEvent, Scope, TuiCommand, crate::keymap::KeyCategory>;

/// Top-level application state and event loop.
#[derive(Debug)]
pub struct TuiApp {
    /// Shared domain state (chat history, extensions).
    pub state: State,
    /// Ephemeral TUI state (input buffer, scroll offset).
    pub tui_state: TuiState,
    /// Message channel for the event loop.
    pub events: MsgHandler,
    /// Which-key keybinding system state.
    #[debug(skip)]
    pub which_key: WhichKeyInstance,
    /// Deferred suspend action queue (e.g., for external editor).
    pub suspend: Suspend,
    /// Background event stream task handle. Set by [`run`](crate::run::run).
    #[debug(skip)]
    pub event_task: Option<tokio::task::JoinHandle<()>>,
    /// Runtime services (tokio handle, extension host). Set during startup.
    pub services: Option<Services>,
    /// Whether the application should exit.
    pub should_quit: bool,
    /// Current application lifecycle status.
    pub status: AppStatus,
}

impl TuiApp {
    /// Creates a new application with default state.
    #[must_use]
    pub fn new() -> Self {
        let keymap = keymap::init();
        let which_key = WhichKeyInstance::new(keymap, Scope::Normal);

        Self {
            state: State::new(nullslop_core::AppData::new()),
            tui_state: TuiState::new(),
            events: MsgHandler::new(),
            which_key,
            suspend: Suspend::new(),
            event_task: None,
            services: None,
            should_quit: false,
            status: AppStatus::Starting,
        }
    }

    /// Processes a single message.
    pub fn handle_msg(&mut self, msg: Msg) {
        match msg {
            Msg::Tick => {}
            Msg::Input(event) => {
                self.handle_input(&event);
            }
            Msg::Command(cmd) => {
                command::dispatch(self, &cmd);
            }
            Msg::ExtensionCommand(cmd) => {
                self.handle_extension_command(cmd);
            }
            Msg::ExtensionsReady(registrations) => {
                for reg in registrations {
                    self.state.write().extensions.register(reg);
                }
                tracing::info!("extensions ready");
            }
        }
    }

    /// Handles a command received from an extension.
    fn handle_extension_command(&mut self, cmd: nullslop_core::Command) {
        match cmd {
            nullslop_core::Command::Custom { name, args } => {
                if name == "echo"
                    && let Some(text) = args.get("text").and_then(|v| v.as_str())
                {
                    self.state
                        .write()
                        .push_entry(nullslop_core::ChatEntry::system(text));
                }
            }
            _ => {
                tracing::warn!(?cmd, "unhandled extension command");
            }
        }
    }

    /// Handles a crossterm input event.
    fn handle_input(&mut self, event: &crossterm::event::Event) {
        if let crossterm::event::Event::Key(key) = event
            && key.kind == crossterm::event::KeyEventKind::Press
            && let Some(cmd) = self.which_key.handle_key(*key)
        {
            command::dispatch(self, &cmd);
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

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

    use super::*;

    fn key_event(code: KeyCode) -> crossterm::event::Event {
        crossterm::event::Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    #[allow(dead_code)]
    fn key_event_with_kind(code: KeyCode, kind: KeyEventKind) -> crossterm::event::Event {
        crossterm::event::Event::Key(KeyEvent::new_with_kind(code, KeyModifiers::NONE, kind))
    }

    #[test]
    fn app_new_starts_in_normal_scope() {
        // Given a new App.
        let app = TuiApp::new();

        // Then which_key scope is Normal.
        assert_eq!(*app.which_key.scope(), Scope::Normal);
        assert!(!app.should_quit);
    }

    #[test]
    fn app_normal_enter_enters_input() {
        // Given an App in Normal scope.
        let mut app = TuiApp::new();
        assert_eq!(*app.which_key.scope(), Scope::Normal);

        // When pressing Enter.
        app.handle_msg(Msg::Input(key_event(KeyCode::Enter)));

        // Then scope changes to Input.
        assert_eq!(*app.which_key.scope(), Scope::Input);
    }

    #[test]
    fn app_normal_esc_quits() {
        // Given an App in Normal scope.
        let mut app = TuiApp::new();

        // When pressing Esc.
        app.handle_msg(Msg::Input(key_event(KeyCode::Esc)));

        // Then should_quit is true.
        assert!(app.should_quit);
    }

    #[test]
    fn app_input_enter_submits() {
        // Given an App in Input scope with "hello" in buffer.
        let mut app = TuiApp::new();
        app.which_key.set_scope(Scope::Input);
        app.tui_state.input_buffer = "hello".to_string();

        // When pressing Enter.
        app.handle_msg(Msg::Input(key_event(KeyCode::Enter)));

        // Then chat has entry and buffer cleared.
        let guard = app.state.read();
        assert_eq!(guard.chat_history.len(), 1);
        assert_eq!(
            guard.chat_history[0].kind,
            nullslop_core::ChatEntryKind::User("hello".to_string())
        );
        drop(guard);
        assert!(app.tui_state.input_buffer.is_empty());
    }

    #[test]
    fn app_input_esc_back_to_normal() {
        // Given an App in Input scope.
        let mut app = TuiApp::new();
        app.which_key.set_scope(Scope::Input);

        // When pressing Esc.
        app.handle_msg(Msg::Input(key_event(KeyCode::Esc)));

        // Then scope is Normal.
        assert_eq!(*app.which_key.scope(), Scope::Normal);
    }

    #[test]
    fn app_input_char_appends() {
        // Given an App in Input scope.
        let mut app = TuiApp::new();
        app.which_key.set_scope(Scope::Input);

        // When pressing 'x'.
        app.handle_msg(Msg::Input(key_event(KeyCode::Char('x'))));

        // Then buffer is "x".
        assert_eq!(app.tui_state.input_buffer, "x");
    }

    #[test]
    fn app_input_backspace_deletes() {
        // Given an App in Input scope with "ab" in buffer.
        let mut app = TuiApp::new();
        app.which_key.set_scope(Scope::Input);
        app.tui_state.input_buffer = "ab".to_string();

        // When pressing Backspace.
        app.handle_msg(Msg::Input(key_event(KeyCode::Backspace)));

        // Then buffer is "a".
        assert_eq!(app.tui_state.input_buffer, "a");
    }
}
