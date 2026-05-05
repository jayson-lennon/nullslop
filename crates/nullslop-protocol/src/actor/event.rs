//! Actor lifecycle events.

use serde::{Deserialize, Serialize};

use crate::EventMsg;

/// An actor is starting up.
#[derive(Debug, Clone, Serialize, Deserialize, EventMsg)]
#[event_msg("actor")]
pub struct ActorStarting {
    /// The actor's name.
    pub name: String,
    /// A short human-readable description of what the actor does.
    pub description: Option<String>,
}

/// An actor has finished starting up.
#[derive(Debug, Clone, Serialize, Deserialize, EventMsg)]
#[event_msg("actor")]
pub struct ActorStarted {
    /// The actor's name.
    pub name: String,
    /// A short human-readable description of what the actor does.
    pub description: Option<String>,
}

/// An actor has completed shutdown.
#[derive(Debug, Clone, Serialize, Deserialize, EventMsg)]
#[event_msg("actor")]
pub struct ActorShutdownCompleted {
    /// The actor's name.
    pub name: String,
}
