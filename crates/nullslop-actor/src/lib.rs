//! nullslop-actor: SDK for building nullslop actors.
//!
//! Actor authors implement the [`Actor`] trait. The host crate
//! (`nullslop-actor-host`) manages lifecycle, bus routing, and run loops.
//!
//! # Core types
//!
//! - [`Actor`] — async trait that actor authors implement
//! - [`ActorRef<M>`] — typed, cloneable handle for sending messages to an actor
//! - [`ActorEnvelope<M>`] — wrapper for all messages an actor can receive
//! - [`ActorContext`] — subscriptions, peer refs, and message sink
//! - [`MessageSink`] — trait for sending bus messages from actors to the application

pub mod actor;
pub mod actor_ref;
pub mod context;
pub mod envelope;
pub mod error;
pub mod message_sink;

pub use actor::Actor;
pub use actor_ref::ActorRef;
pub use context::ActorContext;
pub use envelope::ActorEnvelope;
pub use error::{ActorSendError, SendResult};
pub use message_sink::MessageSink;
