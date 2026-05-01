//! Handles `PushChatEntry` commands to record chat entries in the conversation history.

use crate::AppState;
use npr::CommandAction;
use npr::chat_input::PushChatEntry;
use nullslop_component_core::{Out, define_handler};
use nullslop_protocol as npr;

define_handler! {
    pub(crate) struct ChatLogHandler;

    commands {
        PushChatEntry: on_push_chat_entry,
    }

    events {}
}

impl ChatLogHandler {
    fn on_push_chat_entry(
        cmd: &PushChatEntry,
        state: &mut AppState,
        out: &mut Out,
    ) -> CommandAction {
        state.push_entry(cmd.entry.clone());

        out.submit_event(npr::Event::ChatEntrySubmitted {
            payload: npr::chat_input::ChatEntrySubmitted {
                entry: cmd.entry.clone(),
            },
        });

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
            payload: PushChatEntry { entry },
        });
        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then chat_history has one entry.
        assert_eq!(state.chat_history.len(), 1);
        assert_eq!(
            state.chat_history[0].kind,
            npr::ChatEntryKind::User("hello".to_string())
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
            payload: PushChatEntry { entry },
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
            payload: PushChatEntry { entry },
        });
        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then chat_history has an Actor entry.
        assert_eq!(state.chat_history.len(), 1);
        assert_eq!(
            state.chat_history[0].kind,
            npr::ChatEntryKind::Actor {
                source: "nullslop-echo".to_string(),
                text: "HELLO".to_string(),
            }
        );
    }
}
