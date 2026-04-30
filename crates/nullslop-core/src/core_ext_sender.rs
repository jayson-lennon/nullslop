//! Core extension host sender adapter.
//!
//! [`CoreExtSender`] adapts a `kanal::Sender<AppMsg>` to the
//! [`ExtHostSender`] trait. Used by both TUI and headless modes
//! to bridge extension events into the application message channel.

use crate::{AppMsg, ExtHostSender, RegisteredExtension};
use nullslop_protocol::{Command, Event};

/// Sender that bridges the extension host into an `AppCore` message channel.
///
/// Required because Rust's orphan rules prevent implementing
/// [`ExtHostSender`] on `kanal::Sender<AppMsg>` directly.
pub struct CoreExtSender(kanal::Sender<AppMsg>);

impl CoreExtSender {
    /// Creates a new sender wrapping the given channel.
    #[must_use]
    pub fn new(sender: kanal::Sender<AppMsg>) -> Self {
        Self(sender)
    }
}

impl ExtHostSender for CoreExtSender {
    fn send_extensions_ready(&self, registrations: Vec<RegisteredExtension>) {
        let _ = self.0.send(AppMsg::ExtensionsReady(registrations));
    }

    fn send_command(&self, command: Command, _source: Option<&str>) {
        let _ = self.0.send(AppMsg::Command {
            command,
            source: None,
        });
    }

    fn send_extension_event(&self, event: Event, _source: Option<&str>) {
        let _ = self.0.send(AppMsg::Event {
            event,
            source: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegisteredExtension;

    #[test]
    fn send_extensions_ready_delivers_message() {
        // Given a CoreExtSender wired to a channel.
        let (tx, rx) = kanal::unbounded();
        let sender = CoreExtSender::new(tx);

        // When sending ExtensionsReady.
        let reg = RegisteredExtension {
            name: "test".to_string(),
            commands: vec![],
            subscriptions: vec![],
        };
        sender.send_extensions_ready(vec![reg]);

        // Then the message is received.
        let msg = rx.try_recv().unwrap();
        assert!(matches!(msg, Some(AppMsg::ExtensionsReady(r)) if r.len() == 1));
    }

    #[test]
    fn send_command_delivers_message() {
        // Given a CoreExtSender wired to a channel.
        let (tx, rx) = kanal::unbounded();
        let sender = CoreExtSender::new(tx);

        // When sending a command.
        sender.send_command(Command::AppQuit, None);

        // Then the message is received.
        let msg = rx.try_recv().unwrap();
        assert!(matches!(msg, Some(AppMsg::Command { .. })));
    }

    #[test]
    fn send_extension_event_delivers_message() {
        // Given a CoreExtSender wired to a channel.
        let (tx, rx) = kanal::unbounded();
        let sender = CoreExtSender::new(tx);

        // When sending an event.
        sender.send_extension_event(Event::EventApplicationReady, None);

        // Then the message is received.
        let msg = rx.try_recv().unwrap();
        assert!(matches!(msg, Some(AppMsg::Event { .. })));
    }
}
