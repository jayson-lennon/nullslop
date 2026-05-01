//! Bridge from actor output to the `AppCore` message channel.

use nullslop_actor::{MessageSink, error::SendResult};
use nullslop_protocol::{Command, Event};

use crate::AppMsg;

/// Bridges actor output into `AppCore`'s kanal channel.
///
/// Implements [`MessageSink`] by wrapping actor-originated commands and events
/// in [`AppMsg`] and sending them through the core's message channel.
pub struct ActorMessageSink {
    sender: kanal::Sender<AppMsg>,
}

impl ActorMessageSink {
    /// Creates a new sink wrapping the given channel sender.
    #[must_use]
    pub fn new(sender: kanal::Sender<AppMsg>) -> Self {
        Self { sender }
    }
}

impl MessageSink for ActorMessageSink {
    fn send_command(&self, command: Command) -> SendResult {
        self.sender
            .send(AppMsg::Command {
                command,
                source: None,
            })
            .map_err(|_| nullslop_actor::error::ActorSendError)?;
        Ok(())
    }

    fn send_event(&self, event: Event) -> SendResult {
        self.sender
            .send(AppMsg::Event {
                event,
                source: None,
            })
            .map_err(|_| nullslop_actor::error::ActorSendError)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actor_sink_send_command_delivers_message() {
        // Given an ActorMessageSink wired to a channel.
        let (tx, rx) = kanal::unbounded();
        let sink = ActorMessageSink::new(tx);

        // When sending a command.
        sink.send_command(Command::Quit)
            .expect("send should succeed");

        // Then the message is received as AppMsg::Command.
        let msg = rx
            .try_recv()
            .expect("recv should succeed")
            .expect("should have value");
        assert!(matches!(msg, AppMsg::Command { command, .. } if matches!(command, Command::Quit)));
    }

    #[test]
    fn actor_sink_send_event_delivers_message() {
        // Given an ActorMessageSink wired to a channel.
        let (tx, rx) = kanal::unbounded();
        let sink = ActorMessageSink::new(tx);

        // When sending an event.
        sink.send_event(Event::ApplicationReady)
            .expect("send should succeed");

        // Then the message is received as AppMsg::Event.
        let msg = rx
            .try_recv()
            .expect("recv should succeed")
            .expect("should have value");
        assert!(
            matches!(msg, AppMsg::Event { event, .. } if matches!(event, Event::ApplicationReady))
        );
    }

    #[test]
    fn actor_sink_returns_error_on_closed_channel() {
        // Given an ActorMessageSink with a dropped receiver.
        let (tx, rx) = kanal::unbounded();
        let sink = ActorMessageSink::new(tx);
        drop(rx);

        // When sending a command.
        let result = sink.send_command(Command::Quit);

        // Then it returns an error.
        assert!(result.is_err());
    }
}
