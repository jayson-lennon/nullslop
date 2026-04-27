//! Plugin for chat input commands.
//!
//! Handles inserting characters, deleting graphemes, and submitting messages
//! in the chat input buffer.

use nullslop_plugin::{Out, define_plugin};
use nullslop_protocol::CommandAction;
use nullslop_protocol::command::{ChatBoxDeleteGrapheme, ChatBoxInsertChar, ChatBoxSubmitMessage};

define_plugin! {
    /// Handles chat input commands.
    pub(crate) struct InputModePlugin;

    commands {
        ChatBoxInsertChar: on_insert_char,
        ChatBoxDeleteGrapheme: on_delete_grapheme,
        ChatBoxSubmitMessage: on_submit_message,
    }

    events {}
}

impl InputModePlugin {
    #[allow(clippy::unused_self, clippy::trivially_copy_pass_by_ref)]
    fn on_insert_char(
        &self,
        cmd: &ChatBoxInsertChar,
        state: &mut nullslop_protocol::AppData,
        _out: &mut Out,
    ) -> CommandAction {
        state.input_buffer.push(cmd.ch);
        CommandAction::Continue
    }

    #[allow(clippy::unused_self, clippy::trivially_copy_pass_by_ref)]
    fn on_delete_grapheme(
        &self,
        _cmd: &ChatBoxDeleteGrapheme,
        state: &mut nullslop_protocol::AppData,
        _out: &mut Out,
    ) -> CommandAction {
        state.pop_grapheme();
        CommandAction::Continue
    }

    #[allow(clippy::unused_self, clippy::trivially_copy_pass_by_ref)]
    fn on_submit_message(
        &self,
        _cmd: &ChatBoxSubmitMessage,
        state: &mut nullslop_protocol::AppData,
        out: &mut Out,
    ) -> CommandAction {
        let text = state.input_buffer.clone();
        if !text.is_empty() {
            let entry = nullslop_protocol::ChatEntry::user(&text);
            state.push_entry(entry.clone());
            state.input_buffer.clear();

            out.submit_event(nullslop_protocol::Event::EventChatMessageSubmitted {
                payload: nullslop_protocol::event::EventChatMessageSubmitted { entry },
            });
        }
        CommandAction::Continue
    }
}

#[cfg(test)]
mod tests {
    use nullslop_plugin::Bus;
    use nullslop_protocol::Command;
    use nullslop_protocol::command::{ChatBoxInsertChar, ChatBoxSubmitMessage};

    use super::*;

    #[test]
    fn insert_char_appends_to_buffer() {
        // Given a bus with InputModePlugin registered.
        let mut bus = Bus::new();
        InputModePlugin.register(&mut bus);

        // When processing ChatBoxInsertChar('x').
        bus.submit_command(Command::ChatBoxInsertChar {
            payload: ChatBoxInsertChar { ch: 'x' },
        });
        let mut state = nullslop_protocol::AppData::new();
        bus.process_commands(&mut state);

        // Then input_buffer contains "x".
        assert_eq!(state.input_buffer, "x");
    }

    #[test]
    fn delete_grapheme_removes_last() {
        // Given a bus with InputModePlugin registered.
        let mut bus = Bus::new();
        InputModePlugin.register(&mut bus);

        // When processing ChatBoxInsertChar('a') then ChatBoxInsertChar('b') then ChatBoxDeleteGrapheme.
        bus.submit_command(Command::ChatBoxInsertChar {
            payload: ChatBoxInsertChar { ch: 'a' },
        });
        bus.submit_command(Command::ChatBoxInsertChar {
            payload: ChatBoxInsertChar { ch: 'b' },
        });
        bus.submit_command(Command::ChatBoxDeleteGrapheme);
        let mut state = nullslop_protocol::AppData::new();
        bus.process_commands(&mut state);

        // Then input_buffer is "a".
        assert_eq!(state.input_buffer, "a");
    }

    #[test]
    fn submit_message_adds_entry_and_clears_buffer() {
        // Given a bus with InputModePlugin registered and "hello" in buffer.
        let mut bus = Bus::new();
        InputModePlugin.register(&mut bus);

        let mut state = nullslop_protocol::AppData::new();
        state.input_buffer = "hello".to_string();

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
            nullslop_protocol::ChatEntryKind::User("hello".to_string())
        );
        assert!(state.input_buffer.is_empty());
    }

    #[test]
    fn submit_message_ignores_empty_buffer() {
        // Given a bus with InputModePlugin registered and empty buffer.
        let mut bus = Bus::new();
        InputModePlugin.register(&mut bus);

        // When processing ChatBoxSubmitMessage with empty buffer.
        bus.submit_command(Command::ChatBoxSubmitMessage {
            payload: ChatBoxSubmitMessage {
                text: String::new(),
            },
        });
        let mut state = nullslop_protocol::AppData::new();
        bus.process_commands(&mut state);

        // Then no entry is added and no event is emitted.
        assert!(state.chat_history.is_empty());
        assert!(!bus.has_pending());
    }

    #[test]
    fn submit_message_emits_event() {
        // Given a bus with InputModePlugin registered and "hello" in buffer.
        let mut bus = Bus::new();
        InputModePlugin.register(&mut bus);

        let mut state = nullslop_protocol::AppData::new();
        state.input_buffer = "hello".to_string();

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
            &processed[0],
            nullslop_protocol::Event::EventChatMessageSubmitted { .. }
        ));
    }
}
