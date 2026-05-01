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

