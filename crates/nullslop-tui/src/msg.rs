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
                session_id: npr::SessionId::new(),
                entry: npr::ChatEntry::user("hello"),
            },
        });

        // When matching on the message.
        match msg {
            Msg::Command(npr::Command::PushChatEntry { payload }) => {
                assert_eq!(
                    payload.entry.kind,
                    npr::ChatEntryKind::User("hello".to_owned())
                );
            }
            _ => panic!("expected Command"),
        }
    }
}
