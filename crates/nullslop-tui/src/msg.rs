//! Message channel for the TUI event loop.
//!
//! Provides a unified message type that merges crossterm terminal events,
//! periodic tick messages, and commands into a single stream.

use nullslop_protocol as npr;

pub mod handler;
pub mod sender;

pub use sender::MsgSender;

/// A unified message from any source.
///
/// Merges crossterm terminal events, periodic tick messages,
/// and commands (from key handling or actors) into a single stream
/// consumed by the main event loop.
#[derive(Debug)]
pub enum Msg {
    /// Periodic tick for render refresh.
    Tick,
    /// A crossterm terminal event (key press, resize, etc.).
    Input(crossterm::event::Event),
    /// A command from key handling or an actor.
    Command(npr::Command),
}

#[cfg(test)]
mod tests {
    use super::*;
    use npr::chat_input::PushChatEntry;

    #[test]
    fn command_message_carries_command() {
        // Given a Command message with a PushChatEntry.
        let msg = Msg::Command(npr::Command::PushChatEntry {
            payload: PushChatEntry {
                entry: npr::ChatEntry::user("hello"),
            },
        });

        // Then it matches and the entry is accessible.
        match msg {
            Msg::Command(npr::Command::PushChatEntry { payload }) => {
                assert_eq!(
                    payload.entry.kind,
                    npr::ChatEntryKind::User("hello".to_string())
                );
            }
            _ => panic!("expected Command"),
        }
    }

    #[test]
    fn tick_message_constructs() {
        // Given a Tick message.
        let msg = Msg::Tick;

        // Then it matches Tick.
        assert!(matches!(msg, Msg::Tick));
    }

    #[test]
    fn input_message_wraps_crossterm_event() {
        // Given an Input message wrapping a crossterm key event.
        let event = crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('a'),
            crossterm::event::KeyModifiers::NONE,
        ));
        let msg = Msg::Input(event);

        // Then it matches Input.
        assert!(matches!(msg, Msg::Input(_)));
    }
}
