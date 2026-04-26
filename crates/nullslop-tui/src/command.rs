//! Internal TUI commands dispatched by key handling.
//!
//! These are separate from [`nullslop_core::Command`] which is the
//! extension wire-protocol command type. `TuiCommand` represents
//! internal application actions triggered by key presses.

use std::fmt;

use crate::scope::Scope;
use crate::suspend::SuspendAction;
use crate::{AppStatus, TuiApp};

/// An internal TUI command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuiCommand {
    /// Submit the current input buffer as a chat message.
    SubmitChat,
    /// Quit the application.
    Quit,
    /// Enter input mode (focus input buffer).
    EnterInput,
    /// Return to normal mode from input mode.
    BackToNormal,
    /// Insert a character into the input buffer.
    InsertChar(char),
    /// Delete the last grapheme from the input buffer.
    DeleteGrapheme,
    /// Toggle the which-key popup.
    ToggleWhichKey,
    /// Open external editor for input buffer.
    EditInput,
    /// Set the input buffer to the given content (e.g., from external editor).
    SetInputBuffer(String),
}

impl fmt::Display for TuiCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TuiCommand::SubmitChat => write!(f, "submit chat"),
            TuiCommand::Quit => write!(f, "quit"),
            TuiCommand::EnterInput => write!(f, "enter input"),
            TuiCommand::BackToNormal => write!(f, "back to normal"),
            TuiCommand::InsertChar(c) => write!(f, "insert '{c}'"),
            TuiCommand::DeleteGrapheme => write!(f, "delete"),
            TuiCommand::ToggleWhichKey => write!(f, "toggle which-key"),
            TuiCommand::EditInput => write!(f, "edit in $EDITOR"),
            TuiCommand::SetInputBuffer(_) => write!(f, "set input buffer"),
        }
    }
}

/// Dispatches a TUI command by mutating application state.
pub fn dispatch(app: &mut TuiApp, cmd: &TuiCommand) {
    match cmd {
        TuiCommand::SubmitChat => {
            let text = app.tui_state.input_buffer.clone();
            if !text.is_empty() {
                let entry = nullslop_core::ChatEntry::user(&text);
                app.state.write().push_entry(entry.clone());
                app.tui_state.input_buffer.clear();

                // Broadcast to extensions.
                if let Some(ext) = app.services.as_ref().and_then(|s| s.ext_host()) {
                    ext.send_event(&nullslop_core::Event::NewChatEntry { entry });
                }
            }
        }
        TuiCommand::Quit => {
            app.should_quit = true;
        }
        TuiCommand::EnterInput => {
            app.which_key.set_scope(Scope::Input);
            app.status = AppStatus::Ready;
        }
        TuiCommand::BackToNormal => {
            app.which_key.set_scope(Scope::Normal);
        }
        TuiCommand::InsertChar(c) => {
            app.tui_state.push_char(*c);
        }
        TuiCommand::DeleteGrapheme => {
            app.tui_state.pop_grapheme();
        }
        TuiCommand::ToggleWhichKey => {
            app.which_key.toggle();
        }
        TuiCommand::EditInput => {
            let initial_content = app.tui_state.input_buffer.clone();
            app.suspend.request(SuspendAction::Edit {
                initial_content,
                on_result: Box::new(|result| result.map(TuiCommand::SetInputBuffer)),
            });
        }
        TuiCommand::SetInputBuffer(content) => {
            app.tui_state.input_buffer.clone_from(content);
        }
    }
}

#[cfg(test)]
mod tests {
    use nullslop_core::{ChatEntryKind, State};

    use crate::suspend::SuspendAction;
    use crate::{MsgHandler, TuiState};

    use super::*;

    fn test_app() -> TuiApp {
        TuiApp {
            state: State::new(nullslop_core::AppData::new()),
            tui_state: TuiState::new(),
            events: MsgHandler::new(),
            which_key: crate::app::WhichKeyInstance::new(crate::keymap::init(), Scope::Normal),
            suspend: crate::suspend::Suspend::new(),
            event_task: None,
            services: None,
            should_quit: false,
            status: AppStatus::Starting,
        }
    }

    #[test]
    fn dispatch_submit_chat_adds_entry() {
        // Given an App with "hello" in input_buffer.
        let mut app = test_app();
        app.tui_state.input_buffer = "hello".to_string();

        // When dispatching SubmitChat.
        dispatch(&mut app, &TuiCommand::SubmitChat);

        // Then chat_history has a User entry and buffer is cleared.
        let guard = app.state.read();
        assert_eq!(guard.chat_history.len(), 1);
        assert_eq!(
            guard.chat_history[0].kind,
            ChatEntryKind::User("hello".to_string())
        );
        drop(guard);
        assert!(app.tui_state.input_buffer.is_empty());
    }

    #[test]
    fn dispatch_submit_chat_ignores_empty() {
        // Given an App with empty input_buffer.
        let mut app = test_app();

        // When dispatching SubmitChat.
        dispatch(&mut app, &TuiCommand::SubmitChat);

        // Then chat_history is still empty.
        let guard = app.state.read();
        assert!(guard.chat_history.is_empty());
    }

    #[test]
    fn dispatch_quit_sets_flag() {
        // Given an App.
        let mut app = test_app();

        // When dispatching Quit.
        dispatch(&mut app, &TuiCommand::Quit);

        // Then should_quit is true.
        assert!(app.should_quit);
    }

    #[test]
    fn dispatch_enter_input_sets_scope() {
        // Given an App in Normal scope.
        let mut app = test_app();
        assert_eq!(*app.which_key.scope(), Scope::Normal);

        // When dispatching EnterInput.
        dispatch(&mut app, &TuiCommand::EnterInput);

        // Then which_key scope is Input and status is Ready.
        assert_eq!(*app.which_key.scope(), Scope::Input);
        assert_eq!(app.status, AppStatus::Ready);
    }

    #[test]
    fn dispatch_back_to_normal_sets_scope() {
        // Given an App in Input scope.
        let mut app = test_app();
        app.which_key.set_scope(Scope::Input);

        // When dispatching BackToNormal.
        dispatch(&mut app, &TuiCommand::BackToNormal);

        // Then which_key scope is Normal.
        assert_eq!(*app.which_key.scope(), Scope::Normal);
    }

    #[test]
    fn dispatch_insert_char_appends() {
        // Given an App.
        let mut app = test_app();

        // When dispatching InsertChar('x').
        dispatch(&mut app, &TuiCommand::InsertChar('x'));

        // Then input buffer contains "x".
        assert_eq!(app.tui_state.input_buffer, "x");
    }

    #[test]
    fn dispatch_delete_grapheme_removes() {
        // Given an App with "ab" in buffer.
        let mut app = test_app();
        app.tui_state.input_buffer = "ab".to_string();

        // When dispatching DeleteGrapheme.
        dispatch(&mut app, &TuiCommand::DeleteGrapheme);

        // Then buffer is "a".
        assert_eq!(app.tui_state.input_buffer, "a");
    }

    #[test]
    fn dispatch_toggle_which_key_activates() {
        // Given an App with inactive which_key.
        let mut app = test_app();
        assert!(!app.which_key.active);

        // When dispatching ToggleWhichKey.
        dispatch(&mut app, &TuiCommand::ToggleWhichKey);

        // Then which_key is active.
        assert!(app.which_key.active);
    }

    #[test]
    fn dispatch_edit_input_requests_suspend() {
        // Given an App with "hello" in buffer.
        let mut app = test_app();
        app.tui_state.input_buffer = "hello".to_string();

        // When dispatching EditInput.
        dispatch(&mut app, &TuiCommand::EditInput);

        // Then suspend has a pending action.
        let action = app.suspend.take_action();
        assert!(action.is_some());
    }

    #[test]
    fn dispatch_edit_input_passes_buffer_content() {
        // Given an App with "hello" in buffer.
        let mut app = test_app();
        app.tui_state.input_buffer = "hello".to_string();

        // When dispatching EditInput.
        dispatch(&mut app, &TuiCommand::EditInput);

        // Then the SuspendAction::Edit's initial_content is "hello".
        let action = app.suspend.take_action().expect("should have action");
        match action {
            SuspendAction::Edit {
                initial_content, ..
            } => {
                assert_eq!(initial_content, "hello");
            }
        }
    }

    #[test]
    fn dispatch_set_input_buffer_replaces() {
        // Given an App with "old" in buffer.
        let mut app = test_app();
        app.tui_state.input_buffer = "old".to_string();

        // When dispatching SetInputBuffer("new text").
        dispatch(
            &mut app,
            &TuiCommand::SetInputBuffer("new text".to_string()),
        );

        // Then buffer is "new text".
        assert_eq!(app.tui_state.input_buffer, "new text");
    }

    #[test]
    fn dispatch_edit_input_on_result_maps_content() {
        // Given a SuspendAction::Edit from dispatching EditInput.
        let action = SuspendAction::Edit {
            initial_content: "hello".to_string(),
            on_result: Box::new(|result| result.map(TuiCommand::SetInputBuffer)),
        };

        // When calling on_result with Some("edited").
        let result = match action {
            SuspendAction::Edit { on_result, .. } => on_result(Some("edited".to_string())),
        };

        // Then it returns Some(SetInputBuffer("edited")).
        match result {
            Some(TuiCommand::SetInputBuffer(content)) => assert_eq!(content, "edited"),
            other => panic!("expected SetInputBuffer, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_edit_input_on_result_none_for_unchanged() {
        // Given a SuspendAction::Edit from dispatching EditInput.
        let action = SuspendAction::Edit {
            initial_content: "hello".to_string(),
            on_result: Box::new(|result| result.map(TuiCommand::SetInputBuffer)),
        };

        // When calling on_result with None.
        let result = match action {
            SuspendAction::Edit { on_result, .. } => on_result(None),
        };

        // Then it returns None.
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn dispatch_submit_chat_broadcasts_to_extensions() {
        // Given an App with "hello" in input_buffer and a fake extension host.
        let mut app = test_app();
        app.tui_state.input_buffer = "hello".to_string();
        let fake = std::sync::Arc::new(crate::ext::fake::FakeExtensionHost::new());
        let mut services = crate::services::Services::new(tokio::runtime::Handle::current());
        services
            .register_extension_host(fake.clone() as std::sync::Arc<dyn crate::ext::ExtensionHost>);
        app.services = Some(services);

        // When dispatching SubmitChat.
        dispatch(&mut app, &TuiCommand::SubmitChat);

        // Then the extension host received a NewChatEntry event.
        let events = fake.events_sent();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            nullslop_core::Event::NewChatEntry { .. }
        ));
    }

    #[test]
    fn dispatch_submit_chat_no_broadcast_without_host() {
        // Given an App with "hello" in buffer but no extension host.
        let mut app = test_app();
        app.tui_state.input_buffer = "hello".to_string();

        // When dispatching SubmitChat.
        dispatch(&mut app, &TuiCommand::SubmitChat);

        // Then chat entry is added without error.
        let guard = app.state.read();
        assert_eq!(guard.chat_history.len(), 1);
    }
}
