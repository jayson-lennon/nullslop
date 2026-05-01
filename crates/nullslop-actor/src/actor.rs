//! The actor trait for building nullslop actors.
//!
//! Actor authors implement [`Actor`] with async `handle` and `shutdown` methods.
//! The host creates channels and `ActorRef`s first, then injects them into
//! [`ActorContext`] during activation. After activation, the host spawns a
//! tokio task running the actor's message loop.

use crate::context::ActorContext;
use crate::envelope::ActorEnvelope;
use std::future::Future;

/// Trait for implementing a nullslop actor.
///
/// Actors are activated with a two-phase startup:
/// 1. The host creates `ActorRef` channels for all actors.
/// 2. Each actor's [`activate`](Actor::activate) is called with an [`ActorContext`]
///    pre-loaded with peer `ActorRef` handles.
///
/// After activation, the actor receives all messages — bus events, bus commands,
/// direct messages from other actors, and shutdown — through a single
/// [`ActorEnvelope`] in the [`handle`](Actor::handle) method.
#[allow(async_fn_in_trait)]
pub trait Actor {
    /// The direct message type this actor accepts from other actors.
    type Message: Send + 'static;

    /// Activates the actor. Use `ctx` to subscribe to events/commands
    /// and extract peer `ActorRef` handles.
    ///
    /// This is an associated function (not a method) — it returns `Self`,
    /// constructing the actor during activation.
    fn activate(ctx: &mut ActorContext) -> Self;

    /// Handles an incoming message (event, command, direct, or shutdown).
    fn handle(
        &mut self,
        msg: ActorEnvelope<Self::Message>,
        ctx: &ActorContext,
    ) -> impl Future<Output = ()> + Send;

    /// Shuts down the actor. Called after the run loop exits.
    fn shutdown(self) -> impl Future<Output = ()> + Send;
}
