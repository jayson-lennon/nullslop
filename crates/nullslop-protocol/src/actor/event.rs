//! Actor lifecycle events.

use serde::{Deserialize, Serialize};

use crate::EventMsg;

/// An actor is starting up.
#[derive(Debug, Clone, Serialize, Deserialize, EventMsg)]
#[event_msg("actor")]
pub struct ActorStarting {
    /// The actor's name.
    pub name: String,
}

/// An actor has finished starting up.
#[derive(Debug, Clone, Serialize, Deserialize, EventMsg)]
#[event_msg("actor")]
pub struct ActorStarted {
    /// The actor's name.
    pub name: String,
}

/// An actor has completed shutdown.
#[derive(Debug, Clone, Serialize, Deserialize, EventMsg)]
#[event_msg("actor")]
pub struct ActorShutdownCompleted {
    /// The actor's name.
    pub name: String,
}
