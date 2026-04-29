//! Handler traits for reacting to specific message types.
//!
//! Components implement [`CommandHandler`] or [`EventHandler`] to express interest
//! in particular commands or events. The [`Bus`](crate::Bus) routes each message
//! to every handler registered for that type.

use nullslop_protocol::CommandAction;

use crate::Out;

/// Handler for a specific command type.
///
/// Implementations receive the concrete command and can read or mutate
/// application state. New messages can be submitted via `out`.
///
/// Return [`CommandAction::Stop`] to prevent further handlers from seeing
/// this command. Return [`CommandAction::Continue`] to allow the next handler
/// to run.
pub trait CommandHandler<C: 'static, S> {
    /// Handle a command.
    ///
    /// # Arguments
    ///
    /// * `cmd` - The concrete command to handle.
    /// * `state` - Mutable application state.
    /// * `out` - Buffered output for submitting new commands/events.
    ///
    /// # Returns
    ///
    /// [`CommandAction::Continue`] to allow further handlers,
    /// or [`CommandAction::Stop`] to halt propagation.
    fn handle(&self, cmd: &C, state: &mut S, out: &mut Out) -> CommandAction;
}

/// Handler for a specific event type.
///
/// Implementations receive the concrete event and can read or mutate
/// application state. New messages can be submitted via `out`. Events are
/// fire-and-forget — all registered handlers always run; there is no
/// interception.
pub trait EventHandler<E: 'static, S> {
    /// Handle an event.
    ///
    /// # Arguments
    ///
    /// * `evt` - The concrete event to handle.
    /// * `state` - Mutable application state.
    /// * `out` - Buffered output for submitting new commands/events.
    fn handle(&self, evt: &E, state: &mut S, out: &mut Out);
}
