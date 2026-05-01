//! Actor envelope wrapping all message types into a single channel.
//!
//! Every message an actor processes arrives inside an [`ActorEnvelope`] —
//! whether it originated as a bus event, a bus command, a direct typed message
//! from another actor, or a shutdown signal. Actors match on this enum in their
//! `handle` method.

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
    /// Shutdown signal — the actor should clean up and exit its run loop.
    Shutdown,
}

#[cfg(test)]
mod tests {
    use super::*;
    use nullslop_protocol::{Command, Event};

    #[test]
    fn event_variant_wraps_event() {
        // Given an Event::ApplicationReady.
        let event = Event::ApplicationReady;

        // When wrapped in an ActorEnvelope.
        let envelope: ActorEnvelope<()> = ActorEnvelope::Event(event);

        // Then it matches the Event variant.
        assert!(matches!(
            envelope,
            ActorEnvelope::Event(Event::ApplicationReady)
        ));
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
    fn shutdown_variant_is_unit() {
        // Given a Shutdown envelope.
        let envelope: ActorEnvelope<()> = ActorEnvelope::Shutdown;

        // Then it matches the Shutdown variant.
        assert!(matches!(envelope, ActorEnvelope::Shutdown));
    }
}
