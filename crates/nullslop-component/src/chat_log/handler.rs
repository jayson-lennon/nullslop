//! Handles `PushChatEntry` commands to record chat entries in the conversation history.

use crate::AppState;
use npr::CommandAction;
use npr::chat_input::PushChatEntry;
use npr::system::{MouseScrollDown, MouseScrollUp, ScrollDown, ScrollLineDown, ScrollLineUp, ScrollToBottom, ScrollToTop, ScrollUp};
use nullslop_component_core::{HandlerContext, define_handler};
use nullslop_protocol as npr;
use nullslop_services::Services;

define_handler! {
    pub(crate) struct ChatLogHandler;

    commands {
        PushChatEntry: on_push_chat_entry,
        ScrollUp: on_scroll_up,
        ScrollDown: on_scroll_down,
        MouseScrollUp: on_mouse_scroll_up,
        MouseScrollDown: on_mouse_scroll_down,
        ScrollLineUp: on_scroll_line_up,
        ScrollLineDown: on_scroll_line_down,
        ScrollToTop: on_scroll_to_top,
        ScrollToBottom: on_scroll_to_bottom,
    }

    events {}
}

impl ChatLogHandler {
    /// Number of lines to scroll per keyboard step.
    const SCROLL_STEP: u16 = 10;
    /// Number of lines to scroll per mouse wheel tick.
    const MOUSE_SCROLL_STEP: u16 = 3;
    /// Number of lines to scroll per single-line step (j/k).
    const LINE_SCROLL_STEP: u16 = 1;

    /// Appends a chat entry to the active session's history.
    fn on_push_chat_entry(
        cmd: &PushChatEntry,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        ctx.state.active_session_mut().push_entry(cmd.entry.clone());

        ctx.out.submit_event(npr::Event::ChatEntrySubmitted {
            payload: npr::chat_input::ChatEntrySubmitted {
                session_id: cmd.session_id.clone(),
                entry: cmd.entry.clone(),
            },
        });

        CommandAction::Continue
    }

    /// Scrolls the chat log up by [`SCROLL_STEP`] lines.
    fn on_scroll_up(
        _cmd: &ScrollUp,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        ctx.state.active_session_mut().scroll_up(Self::SCROLL_STEP);
        CommandAction::Continue
    }

    /// Scrolls the chat log down by [`SCROLL_STEP`] lines.
    fn on_scroll_down(
        _cmd: &ScrollDown,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        ctx.state
            .active_session_mut()
            .scroll_down(Self::SCROLL_STEP);
        CommandAction::Continue
    }

    /// Scrolls the chat log up by [`MOUSE_SCROLL_STEP`] lines.
    fn on_mouse_scroll_up(
        _cmd: &MouseScrollUp,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        ctx.state
            .active_session_mut()
            .scroll_up(Self::MOUSE_SCROLL_STEP);
        CommandAction::Continue
    }

    /// Scrolls the chat log down by [`MOUSE_SCROLL_STEP`] lines.
    fn on_mouse_scroll_down(
        _cmd: &MouseScrollDown,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        ctx.state
            .active_session_mut()
            .scroll_down(Self::MOUSE_SCROLL_STEP);
        CommandAction::Continue
    }

    /// Scrolls the chat log up by [`LINE_SCROLL_STEP`] lines.
    fn on_scroll_line_up(
        _cmd: &ScrollLineUp,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        ctx.state
            .active_session_mut()
            .scroll_up(Self::LINE_SCROLL_STEP);
        CommandAction::Continue
    }

    /// Scrolls the chat log down by [`LINE_SCROLL_STEP`] lines.
    fn on_scroll_line_down(
        _cmd: &ScrollLineDown,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        ctx.state
            .active_session_mut()
            .scroll_down(Self::LINE_SCROLL_STEP);
        CommandAction::Continue
    }

    /// Scrolls the chat log to the very top.
    fn on_scroll_to_top(
        _cmd: &ScrollToTop,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        ctx.state.active_session_mut().scroll_to_top();
        CommandAction::Continue
    }

    /// Scrolls the chat log to the very bottom.
    fn on_scroll_to_bottom(
        _cmd: &ScrollToBottom,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        ctx.state.active_session_mut().scroll_to_bottom();
        CommandAction::Continue
    }
}
