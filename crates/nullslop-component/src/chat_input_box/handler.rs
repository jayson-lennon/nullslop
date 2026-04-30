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
    AppSetMode, ChatBoxClear, ChatBoxDeleteGrapheme, ChatBoxDeleteGraphemeForward,
    ChatBoxInsertChar, ChatBoxMoveCursorLeft, ChatBoxMoveCursorRight, ChatBoxMoveCursorToEnd,
    ChatBoxMoveCursorToStart, ChatBoxMoveCursorWordLeft, ChatBoxMoveCursorWordRight,
    ChatBoxSubmitMessage,
};
use nullslop_component_core::{Out, define_handler};
use nullslop_protocol as npr;

define_handler! {
    pub(crate) struct ChatInputBoxHandler;

    commands {
        ChatBoxInsertChar: on_insert_char,
        ChatBoxDeleteGrapheme: on_delete_grapheme,
        ChatBoxDeleteGraphemeForward: on_delete_grapheme_forward,
        ChatBoxSubmitMessage: on_submit_message,
        ChatBoxClear: on_clear,
        ChatBoxMoveCursorLeft: on_move_cursor_left,
        ChatBoxMoveCursorRight: on_move_cursor_right,
        ChatBoxMoveCursorToStart: on_move_cursor_to_start,
        ChatBoxMoveCursorToEnd: on_move_cursor_to_end,
        ChatBoxMoveCursorWordLeft: on_move_cursor_word_left,
        ChatBoxMoveCursorWordRight: on_move_cursor_word_right,
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
        state.chat_input.insert_grapheme_at_cursor(cmd.ch);
        CommandAction::Continue
    }

    fn on_delete_grapheme(
        _cmd: &ChatBoxDeleteGrapheme,
        state: &mut AppState,
        _out: &mut Out,
    ) -> CommandAction {
        state.chat_input.delete_grapheme_before_cursor();
        CommandAction::Continue
    }

    fn on_submit_message(
        _cmd: &ChatBoxSubmitMessage,
        state: &mut AppState,
        out: &mut Out,
    ) -> CommandAction {
        let text = state.chat_input.text().to_string();
        if !text.is_empty() {
            let entry = npr::ChatEntry::user(&text);
            state.push_entry(entry.clone());
            state.chat_input.reset();

            out.submit_event(npr::Event::EventChatMessageSubmitted {
                payload: npr::event::EventChatMessageSubmitted { entry },
            });
        }
        CommandAction::Continue
    }

    fn on_clear(_cmd: &ChatBoxClear, state: &mut AppState, _out: &mut Out) -> CommandAction {
        state.chat_input.reset();
        CommandAction::Continue
    }

    fn on_set_mode(cmd: &AppSetMode, state: &mut AppState, _out: &mut Out) -> CommandAction {
        state.mode = cmd.mode;
        CommandAction::Continue
    }

    fn on_move_cursor_left(
        _cmd: &ChatBoxMoveCursorLeft,
        state: &mut AppState,
        _out: &mut Out,
    ) -> CommandAction {
        state.chat_input.move_cursor_left();
        CommandAction::Continue
    }

    fn on_move_cursor_right(
        _cmd: &ChatBoxMoveCursorRight,
        state: &mut AppState,
        _out: &mut Out,
    ) -> CommandAction {
        state.chat_input.move_cursor_right();
        CommandAction::Continue
    }

    fn on_move_cursor_to_start(
        _cmd: &ChatBoxMoveCursorToStart,
        state: &mut AppState,
        _out: &mut Out,
    ) -> CommandAction {
        state.chat_input.move_cursor_to_start();
        CommandAction::Continue
    }

    fn on_move_cursor_to_end(
        _cmd: &ChatBoxMoveCursorToEnd,
        state: &mut AppState,
        _out: &mut Out,
    ) -> CommandAction {
        state.chat_input.move_cursor_to_end();
        CommandAction::Continue
    }

    fn on_delete_grapheme_forward(
        _cmd: &ChatBoxDeleteGraphemeForward,
        state: &mut AppState,
        _out: &mut Out,
    ) -> CommandAction {
        state.chat_input.delete_grapheme_after_cursor();
        CommandAction::Continue
    }

    fn on_move_cursor_word_left(
        _cmd: &ChatBoxMoveCursorWordLeft,
        state: &mut AppState,
        _out: &mut Out,
    ) -> CommandAction {
        state.chat_input.move_cursor_word_left();
        CommandAction::Continue
    }

    fn on_move_cursor_word_right(
        _cmd: &ChatBoxMoveCursorWordRight,
        state: &mut AppState,
        _out: &mut Out,
    ) -> CommandAction {
        state.chat_input.move_cursor_word_right();
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

        // Then chat_input.text() is "x" and cursor is at 1.
        assert_eq!(state.chat_input.text(), "x");
        assert_eq!(state.chat_input.cursor_pos(), 1);
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

        // Then chat_input.text() is "a" and cursor is at 1.
        assert_eq!(state.chat_input.text(), "a");
        assert_eq!(state.chat_input.cursor_pos(), 1);
    }

    #[test]
    fn submit_message_adds_entry_and_clears_buffer() {
        // Given a bus with ChatInputBoxHandler registered and "hello" in buffer.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        let mut state = AppState::new();
        for ch in "hello".chars() {
            state.chat_input.insert_grapheme_at_cursor(ch);
        }

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
        assert!(state.chat_input.is_empty());
        assert_eq!(state.chat_input.cursor_pos(), 0);
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
        assert_eq!(state.chat_input.text(), "");
    }

    #[test]
    fn submit_message_emits_event() {
        // Given a bus with ChatInputBoxHandler registered and "hello" in buffer.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        let mut state = AppState::new();
        for ch in "hello".chars() {
            state.chat_input.insert_grapheme_at_cursor(ch);
        }

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
        for ch in "some text".chars() {
            state.chat_input.insert_grapheme_at_cursor(ch);
        }

        // When processing ChatBoxClear.
        bus.submit_command(Command::ChatBoxClear);
        bus.process_commands(&mut state);

        // Then the input buffer is empty and cursor is at 0.
        assert!(state.chat_input.is_empty());
        assert_eq!(state.chat_input.cursor_pos(), 0);
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

    // --- Phase 2: bus-level cursor movement tests ---

    #[test]
    fn bus_move_cursor_left_decrements_position() {
        // Given a bus with ChatInputBoxHandler registered and "ab" in buffer.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        bus.submit_command(Command::ChatBoxInsertChar {
            payload: ChatBoxInsertChar { ch: 'a' },
        });
        bus.submit_command(Command::ChatBoxInsertChar {
            payload: ChatBoxInsertChar { ch: 'b' },
        });
        bus.submit_command(Command::ChatBoxMoveCursorLeft);

        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then cursor is at 1 and text is still "ab".
        assert_eq!(state.chat_input.cursor_pos(), 1);
        assert_eq!(state.chat_input.text(), "ab");
    }

    #[test]
    fn bus_move_cursor_right_increments_position() {
        // Given a bus with ChatInputBoxHandler registered and "ab" in buffer.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        bus.submit_command(Command::ChatBoxInsertChar {
            payload: ChatBoxInsertChar { ch: 'a' },
        });
        bus.submit_command(Command::ChatBoxInsertChar {
            payload: ChatBoxInsertChar { ch: 'b' },
        });
        bus.submit_command(Command::ChatBoxMoveCursorLeft);
        bus.submit_command(Command::ChatBoxMoveCursorRight);

        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then cursor is back at 2.
        assert_eq!(state.chat_input.cursor_pos(), 2);
    }

    #[test]
    fn bus_move_cursor_to_start_sets_zero() {
        // Given a bus with ChatInputBoxHandler registered and "abc" in buffer.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        for ch in "abc".chars() {
            bus.submit_command(Command::ChatBoxInsertChar {
                payload: ChatBoxInsertChar { ch },
            });
        }
        bus.submit_command(Command::ChatBoxMoveCursorToStart);

        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then cursor is at 0.
        assert_eq!(state.chat_input.cursor_pos(), 0);
    }

    #[test]
    fn bus_move_cursor_to_end_sets_count() {
        // Given a bus with ChatInputBoxHandler registered and "abc" in buffer.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        for ch in "abc".chars() {
            bus.submit_command(Command::ChatBoxInsertChar {
                payload: ChatBoxInsertChar { ch },
            });
        }
        bus.submit_command(Command::ChatBoxMoveCursorToStart);
        bus.submit_command(Command::ChatBoxMoveCursorToEnd);

        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then cursor is at 3.
        assert_eq!(state.chat_input.cursor_pos(), 3);
    }

    #[test]
    fn bus_delete_grapheme_forward_removes_at_cursor() {
        // Given a bus with ChatInputBoxHandler registered and "ab" in buffer.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        bus.submit_command(Command::ChatBoxInsertChar {
            payload: ChatBoxInsertChar { ch: 'a' },
        });
        bus.submit_command(Command::ChatBoxInsertChar {
            payload: ChatBoxInsertChar { ch: 'b' },
        });
        bus.submit_command(Command::ChatBoxMoveCursorLeft);
        bus.submit_command(Command::ChatBoxDeleteGraphemeForward);

        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then text is "a" and cursor is at 1.
        assert_eq!(state.chat_input.text(), "a");
        assert_eq!(state.chat_input.cursor_pos(), 1);
    }

    #[test]
    fn bus_delete_grapheme_forward_at_end_is_noop() {
        // Given a bus with ChatInputBoxHandler registered and "a" in buffer.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        bus.submit_command(Command::ChatBoxInsertChar {
            payload: ChatBoxInsertChar { ch: 'a' },
        });
        bus.submit_command(Command::ChatBoxDeleteGraphemeForward);

        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then text is still "a".
        assert_eq!(state.chat_input.text(), "a");
    }

    #[test]
    fn bus_move_cursor_word_left_skips_word() {
        // Given a bus with ChatInputBoxHandler registered and "hello world" in buffer.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        for ch in "hello world".chars() {
            bus.submit_command(Command::ChatBoxInsertChar {
                payload: ChatBoxInsertChar { ch },
            });
        }
        bus.submit_command(Command::ChatBoxMoveCursorWordLeft);

        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then cursor is at 6 (start of "world").
        assert_eq!(state.chat_input.cursor_pos(), 6);
    }

    #[test]
    fn bus_move_cursor_word_right_skips_word() {
        // Given a bus with ChatInputBoxHandler registered and "hello world" in buffer.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        for ch in "hello world".chars() {
            bus.submit_command(Command::ChatBoxInsertChar {
                payload: ChatBoxInsertChar { ch },
            });
        }
        bus.submit_command(Command::ChatBoxMoveCursorToStart);
        bus.submit_command(Command::ChatBoxMoveCursorWordRight);

        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then cursor is at 6 (start of "world").
        assert_eq!(state.chat_input.cursor_pos(), 6);
    }
}
