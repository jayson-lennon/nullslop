//! Error types for the actor host.

use wherror::Error;

/// Error type for actor host operations.
#[derive(Debug, Error)]
#[error(debug)]
pub struct ActorHostError;
