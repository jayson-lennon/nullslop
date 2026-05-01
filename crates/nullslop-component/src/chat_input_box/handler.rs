//! Handles user interactions with the chat input box.
//!
//! Responds to typing, deleting, clearing, and submitting messages, as well as
//! switching between normal (browsing) and input (typing) modes.
//!
//! When a message is submitted, it is enqueued via `EnqueueUserMessage` for
//! the message queue handler to dispatch. The input buffer is cleared immediately.

use crate::AppState;
use npr::CommandAction;
use npr::chat_input::{
    Clear, DeleteGrapheme, DeleteGraphemeForward, InsertChar, MoveCursorLeft, MoveCursorRight,
    MoveCursorToEnd, MoveCursorToStart, MoveCursorWordLeft, MoveCursorWordRight, SubmitMessage,
};
use npr::system::SetMode;
use nullslop_component_core::{Out, define_handler};
use nullslop_protocol as npr;

define_handler! {
    pub(crate) struct ChatInputBoxHandler;

    commands {
        InsertChar: on_insert_char,
        DeleteGrapheme: on_delete_grapheme,
        DeleteGraphemeForward: on_delete_grapheme_forward,
        SubmitMessage: on_submit_message,
        Clear: on_clear,
        MoveCursorLeft: on_move_cursor_left,
        MoveCursorRight: on_move_cursor_right,
        MoveCursorToStart: on_move_cursor_to_start,
        MoveCursorToEnd: on_move_cursor_to_end,
        MoveCursorWordLeft: on_move_cursor_word_left,
        MoveCursorWordRight: on_move_cursor_word_right,
        SetMode: on_set_mode,
    }

    events {}
}

impl ChatInputBoxHandler {
    fn on_insert_char(cmd: &InsertChar, state: &mut AppState, _out: &mut Out) -> CommandAction {
        state.active_chat_input_mut().insert_grapheme_at_cursor(cmd.ch);
        CommandAction::Continue
    }

    fn on_delete_grapheme(
        _cmd: &DeleteGrapheme,
        state: &mut AppState,
        _out: &mut Out,
    ) -> CommandAction {
        state.active_chat_input_mut().delete_grapheme_before_cursor();
        CommandAction::Continue
    }

    fn on_submit_message(
        _cmd: &SubmitMessage,
        state: &mut AppState,
        out: &mut Out,
    ) -> CommandAction {
        let text = state.active_chat_input().text().to_string();
        if !text.is_empty() {
            let session_id = state.active_session.clone();
            state.active_chat_input_mut().reset();

            out.submit_command(npr::Command::EnqueueUserMessage {
                payload: npr::chat_input::EnqueueUserMessage { session_id, text },
            });
        }
        CommandAction::Continue
    }

    fn on_clear(_cmd: &Clear, state: &mut AppState, _out: &mut Out) -> CommandAction {
        state.active_chat_input_mut().reset();
        CommandAction::Continue
    }

    fn on_set_mode(cmd: &SetMode, state: &mut AppState, out: &mut Out) -> CommandAction {
        // When leaving Input mode during active streaming, cancel the stream.
        if state.mode == npr::Mode::Input
            && cmd.mode == npr::Mode::Normal
            && !state.active_session().is_idle()
        {
            let session_id = state.active_session.clone();
            out.submit_command(npr::Command::CancelStream {
                payload: npr::provider::CancelStream { session_id },
            });
        }
        state.mode = cmd.mode;
        CommandAction::Continue
    }

    fn on_move_cursor_left(
        _cmd: &MoveCursorLeft,
        state: &mut AppState,
        _out: &mut Out,
    ) -> CommandAction {
        state.active_chat_input_mut().move_cursor_left();
        CommandAction::Continue
    }

    fn on_move_cursor_right(
        _cmd: &MoveCursorRight,
        state: &mut AppState,
        _out: &mut Out,
    ) -> CommandAction {
        state.active_chat_input_mut().move_cursor_right();
        CommandAction::Continue
    }

    fn on_move_cursor_to_start(
        _cmd: &MoveCursorToStart,
        state: &mut AppState,
        _out: &mut Out,
    ) -> CommandAction {
        state.active_chat_input_mut().move_cursor_to_start();
        CommandAction::Continue
    }

    fn on_move_cursor_to_end(
        _cmd: &MoveCursorToEnd,
        state: &mut AppState,
        _out: &mut Out,
    ) -> CommandAction {
        state.active_chat_input_mut().move_cursor_to_end();
        CommandAction::Continue
    }

    fn on_delete_grapheme_forward(
        _cmd: &DeleteGraphemeForward,
        state: &mut AppState,
        _out: &mut Out,
    ) -> CommandAction {
        state.active_chat_input_mut().delete_grapheme_after_cursor();
        CommandAction::Continue
    }

    fn on_move_cursor_word_left(
        _cmd: &MoveCursorWordLeft,
        state: &mut AppState,
        _out: &mut Out,
    ) -> CommandAction {
        state.active_chat_input_mut().move_cursor_word_left();
        CommandAction::Continue
    }

    fn on_move_cursor_word_right(
        _cmd: &MoveCursorWordRight,
        state: &mut AppState,
        _out: &mut Out,
    ) -> CommandAction {
        state.active_chat_input_mut().move_cursor_word_right();
        CommandAction::Continue
    }
}

#[cfg(test)]
mod tests {
    use crate::AppState;
    use npr::Command;
    use npr::chat_input::{InsertChar, SubmitMessage};
    use npr::system::SetMode;
    use nullslop_component_core::Bus;
    use nullslop_protocol as npr;

    use super::*;

    #[test]
    fn insert_char_appends_to_buffer() {
        // Given a bus with ChatInputBoxHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        // When processing InsertChar('x').
        bus.submit_command(Command::InsertChar {
            payload: InsertChar { ch: 'x' },
        });
        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then chat_input.text() is "x" and cursor is at 1.
        assert_eq!(state.active_chat_input().text(), "x");
        assert_eq!(state.active_chat_input().cursor_pos(), 1);
    }

    #[test]
    fn delete_grapheme_removes_last() {
        // Given a bus with ChatInputBoxHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        // When processing InsertChar('a') then InsertChar('b') then DeleteGrapheme.
        bus.submit_command(Command::InsertChar {
            payload: InsertChar { ch: 'a' },
        });
        bus.submit_command(Command::InsertChar {
            payload: InsertChar { ch: 'b' },
        });
        bus.submit_command(Command::DeleteGrapheme);
        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then chat_input.text() is "a" and cursor is at 1.
        assert_eq!(state.active_chat_input().text(), "a");
        assert_eq!(state.active_chat_input().cursor_pos(), 1);
    }

    #[test]
    fn submit_message_adds_entry_and_clears_buffer() {
        // Given a bus with ChatInputBoxHandler and MessageQueueHandler registered and "hello" in buffer.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);
        crate::provider::request_handler::MessageQueueHandler.register(&mut bus);

        let mut state = AppState::new();
        for ch in "hello".chars() {
            state.active_chat_input_mut().insert_grapheme_at_cursor(ch);
        }

        // When processing SubmitMessage.
        bus.submit_command(Command::SubmitMessage {
            payload: SubmitMessage {
                session_id: npr::SessionId::new(),
                text: String::new(),
            },
        });
        bus.process_commands(&mut state);

        // Then active session history has a User entry and buffer is cleared.
        assert_eq!(state.active_session().history().len(), 1);
        assert_eq!(
            state.active_session().history()[0].kind,
            npr::ChatEntryKind::User("hello".to_string())
        );
        assert!(state.active_chat_input().is_empty());
        assert_eq!(state.active_chat_input().cursor_pos(), 0);
    }

    #[test]
    fn submit_message_ignores_empty_buffer() {
        // Given a bus with ChatInputBoxHandler registered and empty buffer.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        // When processing SubmitMessage with empty buffer.
        bus.submit_command(Command::SubmitMessage {
            payload: SubmitMessage {
                session_id: npr::SessionId::new(),
                text: String::new(),
            },
        });
        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then no entry is added and no event is emitted.
        assert!(state.active_session().history().is_empty());
        assert!(!bus.has_pending());
        assert_eq!(state.active_chat_input().text(), "");
    }

    #[test]
    fn submit_message_emits_send_to_llm() {
        // Given a bus with ChatInputBoxHandler and MessageQueueHandler registered and "hello" in buffer.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);
        crate::provider::request_handler::MessageQueueHandler.register(&mut bus);

        let mut state = AppState::new();
        for ch in "hello".chars() {
            state.active_chat_input_mut().insert_grapheme_at_cursor(ch);
        }

        // When processing SubmitMessage.
        bus.submit_command(Command::SubmitMessage {
            payload: SubmitMessage {
                session_id: npr::SessionId::new(),
                text: String::new(),
            },
        });
        bus.process_commands(&mut state);

        // Then a SendToLlmProvider command was cascaded.
        let commands = bus.drain_processed_commands();
        let send = commands
            .iter()
            .find(|c| matches!(c.command, Command::SendToLlmProvider { .. }));
        assert!(
            send.is_some(),
            "expected SendToLlmProvider command after submit"
        );
    }

    #[test]
    fn clear_empties_buffer() {
        // Given a bus with ChatInputBoxHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        let mut state = AppState::new();
        for ch in "some text".chars() {
            state.active_chat_input_mut().insert_grapheme_at_cursor(ch);
        }

        // When processing Clear.
        bus.submit_command(Command::Clear);
        bus.process_commands(&mut state);

        // Then the input buffer is empty and cursor is at 0.
        assert!(state.active_chat_input().is_empty());
        assert_eq!(state.active_chat_input().cursor_pos(), 0);
    }

    #[test]
    fn set_mode_changes_app_mode() {
        // Given a bus with ChatInputBoxHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        // When processing SetMode(Input).
        bus.submit_command(Command::SetMode {
            payload: SetMode {
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

        bus.submit_command(Command::InsertChar {
            payload: InsertChar { ch: 'a' },
        });
        bus.submit_command(Command::InsertChar {
            payload: InsertChar { ch: 'b' },
        });
        // When processing MoveCursorLeft.
        bus.submit_command(Command::MoveCursorLeft);

        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then cursor is at 1 and text is still "ab".
        assert_eq!(state.active_chat_input().cursor_pos(), 1);
        assert_eq!(state.active_chat_input().text(), "ab");
    }

    #[test]
    fn bus_move_cursor_right_increments_position() {
        // Given a bus with ChatInputBoxHandler registered and "ab" in buffer.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        bus.submit_command(Command::InsertChar {
            payload: InsertChar { ch: 'a' },
        });
        bus.submit_command(Command::InsertChar {
            payload: InsertChar { ch: 'b' },
        });
        bus.submit_command(Command::MoveCursorLeft);
        // When processing MoveCursorRight.
        bus.submit_command(Command::MoveCursorRight);

        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then cursor is back at 2.
        assert_eq!(state.active_chat_input().cursor_pos(), 2);
    }

    #[test]
    fn bus_move_cursor_to_start_sets_zero() {
        // Given a bus with ChatInputBoxHandler registered and "abc" in buffer.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        for ch in "abc".chars() {
            bus.submit_command(Command::InsertChar {
                payload: InsertChar { ch },
            });
        }
        // When processing MoveCursorToStart.
        bus.submit_command(Command::MoveCursorToStart);

        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then cursor is at 0.
        assert_eq!(state.active_chat_input().cursor_pos(), 0);
    }

    #[test]
    fn bus_move_cursor_to_end_sets_count() {
        // Given a bus with ChatInputBoxHandler registered and "abc" in buffer.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        for ch in "abc".chars() {
            bus.submit_command(Command::InsertChar {
                payload: InsertChar { ch },
            });
        }
        bus.submit_command(Command::MoveCursorToStart);
        // When processing MoveCursorToEnd.
        bus.submit_command(Command::MoveCursorToEnd);

        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then cursor is at 3.
        assert_eq!(state.active_chat_input().cursor_pos(), 3);
    }

    #[test]
    fn bus_delete_grapheme_forward_removes_at_cursor() {
        // Given a bus with ChatInputBoxHandler registered and "ab" in buffer.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        bus.submit_command(Command::InsertChar {
            payload: InsertChar { ch: 'a' },
        });
        bus.submit_command(Command::InsertChar {
            payload: InsertChar { ch: 'b' },
        });
        bus.submit_command(Command::MoveCursorLeft);
        // When processing DeleteGraphemeForward.
        bus.submit_command(Command::DeleteGraphemeForward);

        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then text is "a" and cursor is at 1.
        assert_eq!(state.active_chat_input().text(), "a");
        assert_eq!(state.active_chat_input().cursor_pos(), 1);
    }

    #[test]
    fn bus_delete_grapheme_forward_at_end_is_noop() {
        // Given a bus with ChatInputBoxHandler registered and "a" in buffer.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        bus.submit_command(Command::InsertChar {
            payload: InsertChar { ch: 'a' },
        });
        // When processing DeleteGraphemeForward at end of buffer.
        bus.submit_command(Command::DeleteGraphemeForward);

        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then text is still "a".
        assert_eq!(state.active_chat_input().text(), "a");
    }

    #[test]
    fn bus_move_cursor_word_left_skips_word() {
        // Given a bus with ChatInputBoxHandler registered and "hello world" in buffer.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        for ch in "hello world".chars() {
            bus.submit_command(Command::InsertChar {
                payload: InsertChar { ch },
            });
        }
        // When processing MoveCursorWordLeft.
        bus.submit_command(Command::MoveCursorWordLeft);

        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then cursor is at 6 (start of "world").
        assert_eq!(state.active_chat_input().cursor_pos(), 6);
    }

    #[test]
    fn bus_move_cursor_word_right_skips_word() {
        // Given a bus with ChatInputBoxHandler registered and "hello world" in buffer.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        for ch in "hello world".chars() {
            bus.submit_command(Command::InsertChar {
                payload: InsertChar { ch },
            });
        }
        bus.submit_command(Command::MoveCursorToStart);
        // When processing MoveCursorWordRight.
        bus.submit_command(Command::MoveCursorWordRight);

        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then cursor is at 6 (start of "world").
        assert_eq!(state.active_chat_input().cursor_pos(), 6);
    }

    #[test]
    fn set_mode_from_input_to_normal_cancels_when_not_idle() {
        // Given a bus with ChatInputBoxHandler registered and a sending session.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        let mut state = AppState::new();
        state.mode = npr::Mode::Input;
        // Simulate a sending state (not yet streaming).
        state.active_session_mut().begin_sending();

        // When processing SetMode(Normal) — ESC while sending.
        bus.submit_command(Command::SetMode {
            payload: SetMode {
                mode: npr::Mode::Normal,
            },
        });
        bus.process_commands(&mut state);

        // Then a CancelStream command was processed (cascaded from on_set_mode).
        let commands = bus.drain_processed_commands();
        let cancel = commands
            .iter()
            .find(|c| matches!(c.command, Command::CancelStream { .. }));
        assert!(
            cancel.is_some(),
            "expected CancelStream command when ESC during sending"
        );

        // And mode is Normal.
        assert_eq!(state.mode, npr::Mode::Normal);
    }

    #[test]
    fn set_mode_from_input_to_normal_no_cancel_when_idle() {
        // Given a bus with ChatInputBoxHandler registered in Input mode, idle.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        let mut state = AppState::new();
        state.mode = npr::Mode::Input;

        // When processing SetMode(Normal) — ESC while idle.
        bus.submit_command(Command::SetMode {
            payload: SetMode {
                mode: npr::Mode::Normal,
            },
        });
        bus.process_commands(&mut state);

        // Then no CancelStream is emitted (only the SetMode was processed).
        let commands = bus.drain_processed_commands();
        let cancel = commands
            .iter()
            .find(|c| matches!(c.command, Command::CancelStream { .. }));
        assert!(
            cancel.is_none(),
            "should not emit CancelStream when idle"
        );

        // And mode is Normal.
        assert_eq!(state.mode, npr::Mode::Normal);
    }

    #[test]
    fn insert_newline_adds_to_buffer() {
        // Given a bus with ChatInputBoxHandler registered and "hello" in buffer.
        let mut bus: Bus<AppState> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        let mut state = AppState::new();
        for ch in "hello".chars() {
            state.active_chat_input_mut().insert_grapheme_at_cursor(ch);
        }

        // When processing InsertChar('\n').
        bus.submit_command(Command::InsertChar {
            payload: InsertChar { ch: '\n' },
        });
        bus.process_commands(&mut state);

        // Then the buffer contains "hello\n" and cursor is at 6.
        assert_eq!(state.active_chat_input().text(), "hello\n");
        assert_eq!(state.active_chat_input().cursor_pos(), 6);
    }
}
