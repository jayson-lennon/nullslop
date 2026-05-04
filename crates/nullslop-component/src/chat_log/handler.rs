//! Handles `PushChatEntry` commands to record chat entries in the conversation history.

use crate::AppState;
use npr::CommandAction;
use npr::chat_input::PushChatEntry;
use npr::system::{ScrollDown, ScrollUp};
use nullslop_component_core::{HandlerContext, define_handler};
use nullslop_protocol as npr;
use nullslop_services::Services;

define_handler! {
    pub(crate) struct ChatLogHandler;

    commands {
        PushChatEntry: on_push_chat_entry,
        ScrollUp: on_scroll_up,
        ScrollDown: on_scroll_down,
    }

    events {}
}

impl ChatLogHandler {
    /// Number of lines to scroll per step.
    const SCROLL_STEP: u16 = 10;

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
}

