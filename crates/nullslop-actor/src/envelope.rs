//! Actor envelope wrapping all message types into a single channel.
//!
//! Every message an actor processes arrives inside an [`ActorEnvelope`] —
//! whether it originated as a bus event, a bus command, a direct typed message
//! from another actor, a system lifecycle message, or a shutdown signal.
//! Actors match on this enum in their `handle` method.

/// System-level lifecycle messages delivered to every actor.
///
/// These messages bypass the event bus — the actor host sends them directly
/// to all actors regardless of subscriptions. Actors match on these in their
/// `handle` method via [`ActorEnvelope::System`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemMessage {
    /// The application has finished starting up.
    ApplicationReady,
    /// The application is shutting down.
    ApplicationShuttingDown,
}

/// Wrapper for all messages an actor can receive.
///
/// The type parameter `M` is the actor's direct message type (e.g.
/// `LlmPipeDirectMsg`). Each actor reads `ActorEnvelope<M>` from a single
/// kanal channel, giving it one unified match block for all incoming messages.
pub enum ActorEnvelope<M> {
    /// A bus event this actor subscribed to during activation.
    Event(nullslop_protocol::Event),
    /// A bus command this actor registered for during activation.
    Command(nullslop_protocol::Command),
    /// A direct typed message from another actor.
    Direct(M),
    /// A system lifecycle message (delivered to all actors, no subscription needed).
    System(SystemMessage),
    /// Shutdown signal — the actor should clean up and exit its run loop.
    Shutdown,
}

#[cfg(test)]
mod tests {
    use super::*;
    use nullslop_protocol::{Command, Event};

    #[test]
    fn event_variant_wraps_event() {
        // Given a ModeChanged event.
        let event = Event::ModeChanged {
            payload: nullslop_protocol::system::ModeChanged {
                from: nullslop_protocol::Mode::Normal,
                to: nullslop_protocol::Mode::Input,
            },
        };

        // When wrapped in an ActorEnvelope.
        let envelope: ActorEnvelope<()> = ActorEnvelope::Event(event);

        // Then it matches the Event variant.
        assert!(matches!(envelope, ActorEnvelope::Event(Event::ModeChanged { .. })));
    }

    #[test]
    fn command_variant_wraps_command() {
        // Given a Command::Quit.
        let command = Command::Quit;

        // When wrapped in an ActorEnvelope.
        let envelope: ActorEnvelope<()> = ActorEnvelope::Command(command);

        // Then it matches the Command variant.
        assert!(matches!(envelope, ActorEnvelope::Command(Command::Quit)));
    }

    #[test]
    fn direct_variant_wraps_message() {
        // Given a direct message.
        let msg = "hello";

        // When wrapped in an ActorEnvelope.
        let envelope = ActorEnvelope::Direct(msg);

        // Then it matches the Direct variant with the inner value.
        assert!(matches!(envelope, ActorEnvelope::Direct("hello")));
    }

    #[test]
    fn system_variant_wraps_system_message() {
        // Given a SystemMessage::ApplicationReady.
        let msg = SystemMessage::ApplicationReady;

        // When wrapped in an ActorEnvelope.
        let envelope: ActorEnvelope<()> = ActorEnvelope::System(msg);

        // Then it matches the System variant.
        assert!(matches!(
            envelope,
            ActorEnvelope::System(SystemMessage::ApplicationReady)
        ));
    }

    #[test]
    fn shutdown_variant_is_unit() {
        // Given a Shutdown envelope.
        let envelope: ActorEnvelope<()> = ActorEnvelope::Shutdown;

        // When matching on the envelope.
        assert!(matches!(envelope, ActorEnvelope::Shutdown));

        // Then it matches the Shutdown variant.
    }
}
