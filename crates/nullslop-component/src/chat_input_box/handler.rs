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


