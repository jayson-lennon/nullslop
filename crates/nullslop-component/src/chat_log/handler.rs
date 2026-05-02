//! Handles `PushChatEntry` commands to record chat entries in the conversation history.

use crate::AppState;
use npr::CommandAction;
use npr::chat_input::PushChatEntry;
use npr::system::{ScrollDown, ScrollUp};
use nullslop_component_core::{Out, define_handler};
use nullslop_protocol as npr;

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
        state: &mut AppState,
        out: &mut Out,
    ) -> CommandAction {
        state.active_session_mut().push_entry(cmd.entry.clone());

        out.submit_event(npr::Event::ChatEntrySubmitted {
            payload: npr::chat_input::ChatEntrySubmitted {
                session_id: cmd.session_id.clone(),
                entry: cmd.entry.clone(),
            },
        });

        CommandAction::Continue
    }

    /// Scrolls the chat log up by [`SCROLL_STEP`] lines.
    fn on_scroll_up(_cmd: &ScrollUp, state: &mut AppState, _out: &mut Out) -> CommandAction {
        state.active_session_mut().scroll_up(Self::SCROLL_STEP);
        CommandAction::Continue
    }

    /// Scrolls the chat log down by [`SCROLL_STEP`] lines.
    fn on_scroll_down(_cmd: &ScrollDown, state: &mut AppState, _out: &mut Out) -> CommandAction {
        state.active_session_mut().scroll_down(Self::SCROLL_STEP);
        CommandAction::Continue
    }
}

#[cfg(test)]
mod tests {
    use crate::AppState;
    use npr::Command;
    use nullslop_component_core::Bus;
    use nullslop_protocol as npr;

    use super::*;

    #[test]
    fn push_chat_entry_adds_to_history() {
        // Given a bus with ChatLogHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
        ChatLogHandler.register(&mut bus);

        // When processing PushChatEntry with a user entry.
        let entry = npr::ChatEntry::user("hello");
        bus.submit_command(Command::PushChatEntry {
            payload: PushChatEntry {
                session_id: npr::SessionId::new(),
                entry,
            },
        });
        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then the active session history has one entry.
        assert_eq!(state.active_session().history().len(), 1);
        assert_eq!(
            state.active_session().history()[0].kind,
            npr::ChatEntryKind::User("hello".to_owned())
        );
    }

    #[test]
    fn push_chat_entry_emits_event() {
        // Given a bus with ChatLogHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
        ChatLogHandler.register(&mut bus);

        // When processing PushChatEntry.
        let entry = npr::ChatEntry::user("hello");
        bus.submit_command(Command::PushChatEntry {
            payload: PushChatEntry {
                session_id: npr::SessionId::new(),
                entry,
            },
        });
        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then a ChatEntrySubmitted event is queued.
        assert!(bus.has_pending());
        bus.process_events(&mut state);

        let processed = bus.drain_processed_events();
        assert_eq!(processed.len(), 1);
        assert!(matches!(
            &processed[0].event,
            npr::Event::ChatEntrySubmitted { .. }
        ));
    }

    #[test]
    fn push_actor_entry_adds_to_history() {
        // Given a bus with ChatLogHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
        ChatLogHandler.register(&mut bus);

        // When processing PushChatEntry with an actor entry.
        let entry = npr::ChatEntry::actor("nullslop-echo", "HELLO");
        bus.submit_command(Command::PushChatEntry {
            payload: PushChatEntry {
                session_id: npr::SessionId::new(),
                entry,
            },
        });
        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then the active session history has an Actor entry.
        assert_eq!(state.active_session().history().len(), 1);
        assert_eq!(
            state.active_session().history()[0].kind,
            npr::ChatEntryKind::Actor {
                source: "nullslop-echo".to_owned(),
                text: "HELLO".to_owned(),
            }
        );
    }

    #[test]
    fn scroll_up_decrements_session_offset() {
        // Given a bus with ChatLogHandler registered and a session with entries.
        let mut bus: Bus<AppState> = Bus::new();
        ChatLogHandler.register(&mut bus);

        let mut state = AppState::new();
        for i in 0..20 {
            state
                .active_session_mut()
                .push_entry(npr::ChatEntry::user(format!("msg {i}")));
        }
        // Pre-condition: push_entry resets scroll to u16::MAX.
        assert_eq!(state.active_session().scroll_offset(), u16::MAX);

        // When processing ScrollUp.
        bus.submit_command(Command::ScrollUp);
        bus.process_commands(&mut state);

        // Then the offset decreased by SCROLL_STEP (10).
        assert_eq!(state.active_session().scroll_offset(), u16::MAX - 10);
    }

    #[test]
    fn scroll_down_increments_session_offset() {
        // Given a bus with ChatLogHandler registered and a session at offset 0.
        let mut bus: Bus<AppState> = Bus::new();
        ChatLogHandler.register(&mut bus);

        let mut state = AppState::new();
        state
            .active_session_mut()
            .push_entry(npr::ChatEntry::user("hello"));
        // Scroll up to bring offset down from MAX.
        state.active_session_mut().scroll_up(u16::MAX); // saturates to 0
        assert_eq!(state.active_session().scroll_offset(), 0);

        // When processing ScrollDown.
        bus.submit_command(Command::ScrollDown);
        bus.process_commands(&mut state);

        // Then the offset increased by SCROLL_STEP (10).
        assert_eq!(state.active_session().scroll_offset(), 10);
    }
}
