//! Message channel for the TUI event loop.
//!
//! Provides a unified message type that merges crossterm terminal events,
//! periodic tick messages, and internal commands into a single stream.

pub mod handler;
pub mod sender;

pub use sender::MsgSender;

/// A unified message from any source.
///
/// Merges crossterm terminal events, periodic tick messages,
/// extension commands, and internal commands into a single stream
/// consumed by the main event loop.
#[derive(Debug)]
pub enum Msg {
    /// Periodic tick for render refresh.
    Tick,
    /// A crossterm terminal event (key press, resize, etc.).
    Input(crossterm::event::Event),
    /// A command dispatched from key handling.
    Command(crate::TuiCommand),
    /// A command received from an extension process.
    ExtensionCommand(nullslop_core::Command),
    /// All extensions have been discovered, spawned, and registered.
    /// Contains the list of registered extensions to add to state.
    ExtensionsReady(Vec<nullslop_core::RegisteredExtension>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use nullslop_core::Command;

    #[test]
    fn extension_command_carries_command() {
        // Given an ExtensionCommand message with a custom command.
        let msg = Msg::ExtensionCommand(Command::Custom {
            name: "echo".to_string(),
            args: serde_json::json!({"text": "hello"}),
        });

        // Then it matches and the name is accessible.
        match msg {
            Msg::ExtensionCommand(Command::Custom { name, .. }) => {
                assert_eq!(name, "echo");
            }
            _ => panic!("expected ExtensionCommand"),
        }
    }

    #[test]
    fn extensions_ready_carries_registrations() {
        // Given an ExtensionsReady message with empty registrations.
        let msg = Msg::ExtensionsReady(vec![]);

        // Then it matches.
        match msg {
            Msg::ExtensionsReady(regs) => {
                assert!(regs.is_empty());
            }
            _ => panic!("expected ExtensionsReady"),
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
