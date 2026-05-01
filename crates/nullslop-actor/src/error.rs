//! Error types for the nullslop-actor crate.
//!
//! All fallible operations in this crate return [`error_stack::Report<E>`]
//! with domain-specific error types defined here, keeping internal channel
//! details from leaking through the public API.

use wherror::Error;

/// Error returned when a message cannot be delivered to an actor.
///
/// This wraps the underlying channel failure (e.g. the actor's receiver
/// was dropped) in a domain error that does not expose internal types.
#[derive(Debug, Error)]
#[error(debug)]
pub struct ActorSendError;

/// Result alias for actor send operations.
pub type SendResult = Result<(), error_stack::Report<ActorSendError>>;

/// Converts a kanal send failure into a [`SendResult`].
///
/// Attaches a human-readable message to the report for diagnostics.
pub(crate) fn from_kanal_send(
    result: Result<(), kanal::SendError>,
    context: &'static str,
) -> SendResult {
    result.map_err(|_| error_stack::Report::new(ActorSendError).attach(context))
}
