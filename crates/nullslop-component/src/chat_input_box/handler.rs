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
    Clear, DeleteGrapheme, DeleteGraphemeForward, InsertChar, Interrupt, MoveCursorDown,
    MoveCursorLeft, MoveCursorRight, MoveCursorToEnd, MoveCursorToStart, MoveCursorUp,
    MoveCursorWordLeft, MoveCursorWordRight, SubmitMessage,
};
use npr::system::SetMode;
use nullslop_component_core::{HandlerContext, define_handler};
use nullslop_protocol as npr;
use nullslop_services::Services;

define_handler! {
    pub(crate) struct ChatInputBoxHandler;

    commands {
        InsertChar: on_insert_char,
        DeleteGrapheme: on_delete_grapheme,
        DeleteGraphemeForward: on_delete_grapheme_forward,
        SubmitMessage: on_submit_message,
        Clear: on_clear,
        Interrupt: on_interrupt,
        MoveCursorLeft: on_move_cursor_left,
        MoveCursorRight: on_move_cursor_right,
        MoveCursorToStart: on_move_cursor_to_start,
        MoveCursorToEnd: on_move_cursor_to_end,
        MoveCursorWordLeft: on_move_cursor_word_left,
        MoveCursorWordRight: on_move_cursor_word_right,
        MoveCursorUp: on_move_cursor_up,
        MoveCursorDown: on_move_cursor_down,
        SetMode: on_set_mode,
    }

    events {}
}

impl ChatInputBoxHandler {
    /// Inserts a character at the cursor position.
    fn on_insert_char(
        cmd: &InsertChar,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        ctx.state
            .active_chat_input_mut()
            .insert_grapheme_at_cursor(cmd.ch);
        CommandAction::Continue
    }

    /// Deletes the grapheme before the cursor.
    fn on_delete_grapheme(
        _cmd: &DeleteGrapheme,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        ctx.state
            .active_chat_input_mut()
            .delete_grapheme_before_cursor();
        CommandAction::Continue
    }

    /// Submits the current input as a user message.
    fn on_submit_message(
        _cmd: &SubmitMessage,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        let text = ctx.state.active_chat_input().text().to_owned();
        if !text.is_empty() {
            let session_id = ctx.state.active_session.clone();
            ctx.state.active_chat_input_mut().reset();

            ctx.out.submit_command(npr::Command::EnqueueUserMessage {
                payload: npr::chat_input::EnqueueUserMessage { session_id, text },
            });
        }
        CommandAction::Continue
    }

    /// Clears the input buffer and resets the cursor.
    fn on_clear(_cmd: &Clear, ctx: &mut HandlerContext<'_, AppState, Services>) -> CommandAction {
        ctx.state.active_chat_input_mut().reset();
        CommandAction::Continue
    }

    /// Context-sensitive interrupt: clears the input buffer if non-empty, otherwise quits.
    fn on_interrupt(
        _cmd: &Interrupt,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        if ctx.state.active_chat_input().is_empty() {
            ctx.out.submit_command(npr::Command::Quit);
        } else {
            ctx.state.active_chat_input_mut().reset();
        }
        CommandAction::Continue
    }

    /// Sets the application input mode, cancelling active streams when leaving Input mode.
    fn on_set_mode(
        cmd: &SetMode,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        // When leaving Input mode during active streaming, cancel the stream.
        if ctx.state.mode == npr::Mode::Input
            && cmd.mode == npr::Mode::Normal
            && !ctx.state.active_session().is_idle()
        {
            let session_id = ctx.state.active_session.clone();
            ctx.out.submit_command(npr::Command::CancelStream {
                payload: npr::provider::CancelStream { session_id },
            });
        }

        // Reset picker state when entering Picker mode.
        if cmd.mode == npr::Mode::Picker {
            ctx.state.picker.reset();
        }

        ctx.state.mode = cmd.mode;
        CommandAction::Continue
    }

    /// Moves the cursor left one grapheme.
    fn on_move_cursor_left(
        _cmd: &MoveCursorLeft,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        ctx.state.active_chat_input_mut().move_cursor_left();
        CommandAction::Continue
    }

    /// Moves the cursor right one grapheme.
    fn on_move_cursor_right(
        _cmd: &MoveCursorRight,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        ctx.state.active_chat_input_mut().move_cursor_right();
        CommandAction::Continue
    }

    /// Moves the cursor to the start of the input.
    fn on_move_cursor_to_start(
        _cmd: &MoveCursorToStart,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        ctx.state.active_chat_input_mut().move_cursor_to_start();
        CommandAction::Continue
    }

    /// Moves the cursor to the end of the input.
    fn on_move_cursor_to_end(
        _cmd: &MoveCursorToEnd,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        ctx.state.active_chat_input_mut().move_cursor_to_end();
        CommandAction::Continue
    }

    /// Deletes the grapheme after the cursor.
    fn on_delete_grapheme_forward(
        _cmd: &DeleteGraphemeForward,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        ctx.state
            .active_chat_input_mut()
            .delete_grapheme_after_cursor();
        CommandAction::Continue
    }

    /// Moves the cursor left one word.
    fn on_move_cursor_word_left(
        _cmd: &MoveCursorWordLeft,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        ctx.state.active_chat_input_mut().move_cursor_word_left();
        CommandAction::Continue
    }

    /// Moves the cursor right one word.
    fn on_move_cursor_word_right(
        _cmd: &MoveCursorWordRight,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        ctx.state.active_chat_input_mut().move_cursor_word_right();
        CommandAction::Continue
    }

    /// Moves the cursor up one visual line.
    fn on_move_cursor_up(
        _cmd: &MoveCursorUp,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        ctx.state.active_chat_input_mut().move_cursor_up();
        CommandAction::Continue
    }

    /// Moves the cursor down one visual line.
    fn on_move_cursor_down(
        _cmd: &MoveCursorDown,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        ctx.state.active_chat_input_mut().move_cursor_down();
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
    use nullslop_services::Services;

    use super::*;
    use crate::test_utils;

    #[test]
    fn insert_char_appends_to_buffer() {
        // Given a bus with ChatInputBoxHandler registered.
        let mut bus: Bus<AppState, Services> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        // When processing InsertChar('x').
        bus.submit_command(Command::InsertChar {
            payload: InsertChar { ch: 'x' },
        });
        let services = test_utils::test_services();
        let mut state = AppState::default();
        bus.process_commands(&mut state, &services);

        // Then chat_input.text() is "x" and cursor is at 1.
        assert_eq!(state.active_chat_input().text(), "x");
        assert_eq!(state.active_chat_input().cursor_pos(), 1);
    }

    #[test]
    fn delete_grapheme_removes_last() {
        // Given a bus with ChatInputBoxHandler registered.
        let mut bus: Bus<AppState, Services> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        // When processing InsertChar('a') then InsertChar('b') then DeleteGrapheme.
        bus.submit_command(Command::InsertChar {
            payload: InsertChar { ch: 'a' },
        });
        bus.submit_command(Command::InsertChar {
            payload: InsertChar { ch: 'b' },
        });
        bus.submit_command(Command::DeleteGrapheme);
        let services = test_utils::test_services();
        let mut state = AppState::default();
        bus.process_commands(&mut state, &services);

        // Then chat_input.text() is "a" and cursor is at 1.
        assert_eq!(state.active_chat_input().text(), "a");
        assert_eq!(state.active_chat_input().cursor_pos(), 1);
    }

    #[test]
    fn submit_message_adds_entry_and_clears_buffer() {
        // Given a bus with ChatInputBoxHandler and MessageQueueHandler registered and "hello" in buffer.
        let mut bus: Bus<AppState, Services> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);
        crate::provider::request_handler::MessageQueueHandler.register(&mut bus);

        let services = test_utils::test_services();
        let mut state = AppState::default();
        state.active_provider = "test".to_owned();
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
        bus.process_commands(&mut state, &services);

        // Then active session history has a User entry and buffer is cleared.
        assert_eq!(state.active_session().history().len(), 1);
        assert_eq!(
            state.active_session().history()[0].kind,
            npr::ChatEntryKind::User("hello".to_owned())
        );
        assert!(state.active_chat_input().is_empty());
        assert_eq!(state.active_chat_input().cursor_pos(), 0);
    }

    #[test]
    fn submit_message_ignores_empty_buffer() {
        // Given a bus with ChatInputBoxHandler registered and empty buffer.
        let mut bus: Bus<AppState, Services> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        // When processing SubmitMessage with empty buffer.
        bus.submit_command(Command::SubmitMessage {
            payload: SubmitMessage {
                session_id: npr::SessionId::new(),
                text: String::new(),
            },
        });
        let services = test_utils::test_services();
        let mut state = AppState::default();
        bus.process_commands(&mut state, &services);

        // Then no entry is added and no event is emitted.
        assert!(state.active_session().history().is_empty());
        assert!(!bus.has_pending());
        assert_eq!(state.active_chat_input().text(), "");
    }

    #[test]
    fn submit_message_emits_send_to_llm() {
        // Given a bus with ChatInputBoxHandler and MessageQueueHandler registered and "hello" in buffer.
        let mut bus: Bus<AppState, Services> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);
        crate::provider::request_handler::MessageQueueHandler.register(&mut bus);

        let services = test_utils::test_services();
        let mut state = AppState::default();
        state.active_provider = "test".to_owned();
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
        bus.process_commands(&mut state, &services);

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
        let mut bus: Bus<AppState, Services> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        let services = test_utils::test_services();
        let mut state = AppState::default();
        for ch in "some text".chars() {
            state.active_chat_input_mut().insert_grapheme_at_cursor(ch);
        }

        // When processing Clear.
        bus.submit_command(Command::Clear);
        bus.process_commands(&mut state, &services);

        // Then the input buffer is empty and cursor is at 0.
        assert!(state.active_chat_input().is_empty());
        assert_eq!(state.active_chat_input().cursor_pos(), 0);
    }

    #[test]
    fn set_mode_changes_app_mode() {
        // Given a bus with ChatInputBoxHandler registered.
        let mut bus: Bus<AppState, Services> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        // When processing SetMode(Input).
        bus.submit_command(Command::SetMode {
            payload: SetMode {
                mode: npr::Mode::Input,
            },
        });
        let services = test_utils::test_services();
        let mut state = AppState::default();
        bus.process_commands(&mut state, &services);

        // Then state mode is Input.
        assert_eq!(state.mode, npr::Mode::Input);
    }

    // --- Phase 2: bus-level cursor movement tests ---

    #[test]
    fn bus_move_cursor_left_decrements_position() {
        // Given a bus with ChatInputBoxHandler registered and "ab" in buffer.
        let mut bus: Bus<AppState, Services> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        bus.submit_command(Command::InsertChar {
            payload: InsertChar { ch: 'a' },
        });
        bus.submit_command(Command::InsertChar {
            payload: InsertChar { ch: 'b' },
        });
        // When processing MoveCursorLeft.
        bus.submit_command(Command::MoveCursorLeft);

        let services = test_utils::test_services();
        let mut state = AppState::default();
        bus.process_commands(&mut state, &services);

        // Then cursor is at 1 and text is still "ab".
        assert_eq!(state.active_chat_input().cursor_pos(), 1);
        assert_eq!(state.active_chat_input().text(), "ab");
    }

    #[test]
    fn bus_move_cursor_right_increments_position() {
        // Given a bus with ChatInputBoxHandler registered and "ab" in buffer.
        let mut bus: Bus<AppState, Services> = Bus::new();
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

        let services = test_utils::test_services();
        let mut state = AppState::default();
        bus.process_commands(&mut state, &services);

        // Then cursor is back at 2.
        assert_eq!(state.active_chat_input().cursor_pos(), 2);
    }

    #[test]
    fn bus_move_cursor_to_start_sets_zero() {
        // Given a bus with ChatInputBoxHandler registered and "abc" in buffer.
        let mut bus: Bus<AppState, Services> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        for ch in "abc".chars() {
            bus.submit_command(Command::InsertChar {
                payload: InsertChar { ch },
            });
        }
        // When processing MoveCursorToStart.
        bus.submit_command(Command::MoveCursorToStart);

        let services = test_utils::test_services();
        let mut state = AppState::default();
        bus.process_commands(&mut state, &services);

        // Then cursor is at 0.
        assert_eq!(state.active_chat_input().cursor_pos(), 0);
    }

    #[test]
    fn bus_move_cursor_to_end_sets_count() {
        // Given a bus with ChatInputBoxHandler registered and "abc" in buffer.
        let mut bus: Bus<AppState, Services> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        for ch in "abc".chars() {
            bus.submit_command(Command::InsertChar {
                payload: InsertChar { ch },
            });
        }
        bus.submit_command(Command::MoveCursorToStart);
        // When processing MoveCursorToEnd.
        bus.submit_command(Command::MoveCursorToEnd);

        let services = test_utils::test_services();
        let mut state = AppState::default();
        bus.process_commands(&mut state, &services);

        // Then cursor is at 3.
        assert_eq!(state.active_chat_input().cursor_pos(), 3);
    }

    #[test]
    fn bus_delete_grapheme_forward_removes_at_cursor() {
        // Given a bus with ChatInputBoxHandler registered and "ab" in buffer.
        let mut bus: Bus<AppState, Services> = Bus::new();
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

        let services = test_utils::test_services();
        let mut state = AppState::default();
        bus.process_commands(&mut state, &services);

        // Then text is "a" and cursor is at 1.
        assert_eq!(state.active_chat_input().text(), "a");
        assert_eq!(state.active_chat_input().cursor_pos(), 1);
    }

    #[test]
    fn bus_delete_grapheme_forward_at_end_is_noop() {
        // Given a bus with ChatInputBoxHandler registered and "a" in buffer.
        let mut bus: Bus<AppState, Services> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        bus.submit_command(Command::InsertChar {
            payload: InsertChar { ch: 'a' },
        });
        // When processing DeleteGraphemeForward at end of buffer.
        bus.submit_command(Command::DeleteGraphemeForward);

        let services = test_utils::test_services();
        let mut state = AppState::default();
        bus.process_commands(&mut state, &services);

        // Then text is still "a".
        assert_eq!(state.active_chat_input().text(), "a");
    }

    #[test]
    fn bus_move_cursor_word_left_skips_word() {
        // Given a bus with ChatInputBoxHandler registered and "hello world" in buffer.
        let mut bus: Bus<AppState, Services> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        for ch in "hello world".chars() {
            bus.submit_command(Command::InsertChar {
                payload: InsertChar { ch },
            });
        }
        // When processing MoveCursorWordLeft.
        bus.submit_command(Command::MoveCursorWordLeft);

        let services = test_utils::test_services();
        let mut state = AppState::default();
        bus.process_commands(&mut state, &services);

        // Then cursor is at 6 (start of "world").
        assert_eq!(state.active_chat_input().cursor_pos(), 6);
    }

    #[test]
    fn bus_move_cursor_word_right_skips_word() {
        // Given a bus with ChatInputBoxHandler registered and "hello world" in buffer.
        let mut bus: Bus<AppState, Services> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        for ch in "hello world".chars() {
            bus.submit_command(Command::InsertChar {
                payload: InsertChar { ch },
            });
        }
        bus.submit_command(Command::MoveCursorToStart);
        // When processing MoveCursorWordRight.
        bus.submit_command(Command::MoveCursorWordRight);

        let services = test_utils::test_services();
        let mut state = AppState::default();
        bus.process_commands(&mut state, &services);

        // Then cursor is at 6 (start of "world").
        assert_eq!(state.active_chat_input().cursor_pos(), 6);
    }

    #[test]
    fn set_mode_from_input_to_normal_cancels_when_not_idle() {
        // Given a bus with ChatInputBoxHandler registered and a sending session.
        let mut bus: Bus<AppState, Services> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        let services = test_utils::test_services();
        let mut state = AppState::default();
        state.mode = npr::Mode::Input;
        // Simulate a sending state (not yet streaming).
        state.active_session_mut().begin_sending();

        // When processing SetMode(Normal) — ESC while sending.
        bus.submit_command(Command::SetMode {
            payload: SetMode {
                mode: npr::Mode::Normal,
            },
        });
        bus.process_commands(&mut state, &services);

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
        let mut bus: Bus<AppState, Services> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        let services = test_utils::test_services();
        let mut state = AppState::default();
        state.mode = npr::Mode::Input;

        // When processing SetMode(Normal) — ESC while idle.
        bus.submit_command(Command::SetMode {
            payload: SetMode {
                mode: npr::Mode::Normal,
            },
        });
        bus.process_commands(&mut state, &services);

        // Then no CancelStream is emitted (only the SetMode was processed).
        let commands = bus.drain_processed_commands();
        let cancel = commands
            .iter()
            .find(|c| matches!(c.command, Command::CancelStream { .. }));
        assert!(cancel.is_none(), "should not emit CancelStream when idle");

        // And mode is Normal.
        assert_eq!(state.mode, npr::Mode::Normal);
    }

    #[test]
    fn insert_newline_adds_to_buffer() {
        // Given a bus with ChatInputBoxHandler registered and "hello" in buffer.
        let mut bus: Bus<AppState, Services> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        let services = test_utils::test_services();
        let mut state = AppState::default();
        for ch in "hello".chars() {
            state.active_chat_input_mut().insert_grapheme_at_cursor(ch);
        }

        // When processing InsertChar('\n').
        bus.submit_command(Command::InsertChar {
            payload: InsertChar { ch: '\n' },
        });
        bus.process_commands(&mut state, &services);

        // Then the buffer contains "hello\n" and cursor is at 6.
        assert_eq!(state.active_chat_input().text(), "hello\n");
        assert_eq!(state.active_chat_input().cursor_pos(), 6);
    }

    #[test]
    fn bus_move_cursor_up_moves_to_previous_line() {
        // Given a bus with ChatInputBoxHandler registered and "ab\ncd" in buffer.
        let mut bus: Bus<AppState, Services> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        let services = test_utils::test_services();
        let mut state = AppState::default();
        for ch in "ab\ncd".chars() {
            state.active_chat_input_mut().insert_grapheme_at_cursor(ch);
        }
        // cursor at end (5), row=1, col=2

        // When processing MoveCursorUp.
        bus.submit_command(Command::MoveCursorUp);
        bus.process_commands(&mut state, &services);

        // Then cursor is at row 0, col 2 (grapheme index 2).
        assert_eq!(state.active_chat_input().cursor_row_col(), (0, 2));
    }

    #[test]
    fn bus_move_cursor_down_moves_to_next_line() {
        // Given a bus with ChatInputBoxHandler registered and "ab\ncd" in buffer, cursor at start.
        let mut bus: Bus<AppState, Services> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        let services = test_utils::test_services();
        let mut state = AppState::default();
        for ch in "ab\ncd".chars() {
            state.active_chat_input_mut().insert_grapheme_at_cursor(ch);
        }
        state.active_chat_input_mut().move_cursor_to_start();
        // cursor at 0, row=0, col=0

        // When processing MoveCursorDown.
        bus.submit_command(Command::MoveCursorDown);
        bus.process_commands(&mut state, &services);

        // Then cursor is at row 1, col 0 (grapheme index 3).
        assert_eq!(state.active_chat_input().cursor_row_col(), (1, 0));
    }

    #[test]
    fn bus_move_cursor_up_noop_on_first_line() {
        // Given a bus with ChatInputBoxHandler registered and "hello" (single line).
        let mut bus: Bus<AppState, Services> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        let services = test_utils::test_services();
        let mut state = AppState::default();
        for ch in "hello".chars() {
            state.active_chat_input_mut().insert_grapheme_at_cursor(ch);
        }
        state.active_chat_input_mut().move_cursor_to_start();

        // When processing MoveCursorUp.
        bus.submit_command(Command::MoveCursorUp);
        bus.process_commands(&mut state, &services);

        // Then cursor stays at 0.
        assert_eq!(state.active_chat_input().cursor_pos(), 0);
    }

    #[test]
    fn bus_move_cursor_down_noop_on_last_line() {
        // Given a bus with ChatInputBoxHandler registered and "hello" (single line).
        let mut bus: Bus<AppState, Services> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        let services = test_utils::test_services();
        let mut state = AppState::default();
        for ch in "hello".chars() {
            state.active_chat_input_mut().insert_grapheme_at_cursor(ch);
        }

        // When processing MoveCursorDown.
        bus.submit_command(Command::MoveCursorDown);
        bus.process_commands(&mut state, &services);

        // Then cursor stays at end (5).
        assert_eq!(state.active_chat_input().cursor_pos(), 5);
    }

    // --- Interrupt tests ---

    #[test]
    fn interrupt_clears_buffer_when_non_empty() {
        // Given a bus with ChatInputBoxHandler registered and "hello" in buffer.
        let mut bus: Bus<AppState, Services> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);

        let services = test_utils::test_services();
        let mut state = AppState::default();
        for ch in "hello".chars() {
            state.active_chat_input_mut().insert_grapheme_at_cursor(ch);
        }

        // When processing Interrupt.
        bus.submit_command(Command::Interrupt);
        bus.process_commands(&mut state, &services);

        // Then the buffer is cleared.
        assert!(state.active_chat_input().is_empty());
        assert_eq!(state.active_chat_input().cursor_pos(), 0);
    }

    #[test]
    fn interrupt_quits_when_buffer_empty() {
        // Given a bus with ChatInputBoxHandler and AppQuitHandler registered, empty buffer.
        let mut bus: Bus<AppState, Services> = Bus::new();
        ChatInputBoxHandler.register(&mut bus);
        crate::app_quit::AppQuitHandler.register(&mut bus);

        // When processing Interrupt with empty input.
        bus.submit_command(Command::Interrupt);
        let services = test_utils::test_services();
        let mut state = AppState::default();
        bus.process_commands(&mut state, &services);

        // Then should_quit is true (Interrupt cascaded Quit).
        assert!(state.should_quit);
    }
}
