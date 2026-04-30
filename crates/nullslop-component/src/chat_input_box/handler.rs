//! Handles user interactions with the chat input box.
//!
//! Responds to typing, deleting, clearing, and submitting messages, as well as
//! switching between normal (browsing) and input (typing) modes.
//!
//! When a message is submitted, it is added to the conversation history, the input
//! buffer is cleared, and an event is emitted so other components can react to the
//! new message.

use crate::AppState;
use npr::CommandAction;
use npr::command::{
    AppSetMode, ChatBoxClear, ChatBoxDeleteGrapheme, ChatBoxInsertChar, ChatBoxSubmitMessage,
};
use nullslop_component_core::{Out, define_handler};
use nullslop_protocol as npr;

define_handler! {
    pub(crate) struct ChatInputBoxHandler;

    commands {
        ChatBoxInsertChar: on_insert_char,
        ChatBoxDeleteGrapheme: on_delete_grapheme,
        ChatBoxSubmitMessage: on_submit_message,
        ChatBoxClear: on_clear,
        AppSetMode: on_set_mode,
    }

    events {}
}

impl ChatInputBoxHandler {
    fn on_insert_char(
        cmd: &ChatBoxInsertChar,
        state: &mut AppState,
        _out: &mut Out,
    ) -> CommandAction {
        state.chat_input.input_buffer.push(cmd.ch);
        CommandAction::Continue
    }

    fn on_delete_grapheme(
        _cmd: &ChatBoxDeleteGrapheme,
        state: &mut AppState,
        _out: &mut Out,
    ) -> CommandAction {
        state.chat_input.pop_grapheme();
        CommandAction::Continue
    }

    fn on_submit_message(
        _cmd: &ChatBoxSubmitMessage,
        state: &mut AppState,
        out: &mut Out,
    ) -> CommandAction {
        let text = state.chat_input.input_buffer.clone();
        if !text.is_empty() {
            let entry = npr::ChatEntry::user(&text);
            state.push_entry(entry.clone());
            state.chat_input.input_buffer.clear();

            out.submit_event(npr::Event::EventChatMessageSubmitted {
                payload: npr::event::EventChatMessageSubmitted { entry },
            });
        }
        CommandAction::Continue
    }

    fn on_clear(_cmd: &ChatBoxClear, state: &mut AppState, _out: &mut Out) -> CommandAction {
        state.chat_input.input_buffer.clear();
        CommandAction::Continue
    }

    fn on_set_mode(cmd: &AppSetMode, state: &mut AppState, _out: &mut Out) -> CommandAction {
        state.mode = cmd.mode;
        CommandAction::Continue
    }
}

#[cfg(test)]
mod tests {
    use crate::AppState;
    use npr::Command;
    use npr::command::{AppSetMode, ChatBoxInsertChar, ChatBoxSubmitMessage};
    use nullslop_component_core::Bus;
    use nullslop_protocol as npr;

    use super::*;

    #[test]
    fn insert_char_appends_to_buffer() {
        // Given a bus with ChatInputBoxHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        // When processing ChatBoxInsertChar('x').
        bus.submit_command(Command::ChatBoxInsertChar {
            payload: ChatBoxInsertChar { ch: 'x' },
        });
        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then chat_input.input_buffer contains "x".
        assert_eq!(state.chat_input.input_buffer, "x");
    }

    #[test]
    fn delete_grapheme_removes_last() {
        // Given a bus with ChatInputBoxHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        // When processing ChatBoxInsertChar('a') then ChatBoxInsertChar('b') then ChatBoxDeleteGrapheme.
        bus.submit_command(Command::ChatBoxInsertChar {
            payload: ChatBoxInsertChar { ch: 'a' },
        });
        bus.submit_command(Command::ChatBoxInsertChar {
            payload: ChatBoxInsertChar { ch: 'b' },
        });
        bus.submit_command(Command::ChatBoxDeleteGrapheme);
        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then chat_input.input_buffer is "a".
        assert_eq!(state.chat_input.input_buffer, "a");
    }

    #[test]
    fn submit_message_adds_entry_and_clears_buffer() {
        // Given a bus with ChatInputBoxHandler registered and "hello" in buffer.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        let mut state = AppState::new();
        state.chat_input.input_buffer = "hello".to_string();

        // When processing ChatBoxSubmitMessage.
        bus.submit_command(Command::ChatBoxSubmitMessage {
            payload: ChatBoxSubmitMessage {
                text: String::new(),
            },
        });
        bus.process_commands(&mut state);

        // Then chat_history has a User entry and buffer is cleared.
        assert_eq!(state.chat_history.len(), 1);
        assert_eq!(
            state.chat_history[0].kind,
            npr::ChatEntryKind::User("hello".to_string())
        );
        assert!(state.chat_input.input_buffer.is_empty());
    }

    #[test]
    fn submit_message_ignores_empty_buffer() {
        // Given a bus with ChatInputBoxHandler registered and empty buffer.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        // When processing ChatBoxSubmitMessage with empty buffer.
        bus.submit_command(Command::ChatBoxSubmitMessage {
            payload: ChatBoxSubmitMessage {
                text: String::new(),
            },
        });
        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then no entry is added and no event is emitted.
        assert!(state.chat_history.is_empty());
        assert!(!bus.has_pending());
    }

    #[test]
    fn submit_message_emits_event() {
        // Given a bus with ChatInputBoxHandler registered and "hello" in buffer.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        let mut state = AppState::new();
        state.chat_input.input_buffer = "hello".to_string();

        // When processing ChatBoxSubmitMessage.
        bus.submit_command(Command::ChatBoxSubmitMessage {
            payload: ChatBoxSubmitMessage {
                text: String::new(),
            },
        });
        bus.process_commands(&mut state);

        // Then an event is queued.
        assert!(bus.has_pending());

        // When processing events.
        bus.process_events(&mut state);

        // Then the event is in processed_events.
        let processed = bus.drain_processed_events();
        assert_eq!(processed.len(), 1);
        assert!(matches!(
            &processed[0].event,
            npr::Event::EventChatMessageSubmitted { .. }
        ));
    }

    #[test]
    fn clear_empties_buffer() {
        // Given a bus with ChatInputBoxHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        let mut state = AppState::new();
        state.chat_input.input_buffer = "some text".to_string();

        // When processing ChatBoxClear.
        bus.submit_command(Command::ChatBoxClear);
        bus.process_commands(&mut state);

        // Then the input buffer is empty.
        assert!(state.chat_input.input_buffer.is_empty());
    }

    #[test]
    fn set_mode_changes_app_mode() {
        // Given a bus with ChatInputBoxHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        // When processing AppSetMode(Input).
        bus.submit_command(Command::AppSetMode {
            payload: AppSetMode {
                mode: npr::Mode::Input,
            },
        });
        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then state mode is Input.
        assert_eq!(state.mode, npr::Mode::Input);
    }
}
