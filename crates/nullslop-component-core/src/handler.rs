//! Handler traits and context for reacting to specific message types.
//!
//! Components implement [`CommandHandler`] or [`EventHandler`] to express interest
//! in particular commands or events. The [`Bus`](crate::Bus) routes each message
//! to every handler registered for that type.
//!
//! Handlers receive a [`HandlerContext`] that bundles mutable state, read-only
//! services, and an output buffer into a single parameter.

use nullslop_protocol::CommandAction;

use crate::Out;

/// Bundles mutable state, read-only services, and an output buffer for handler dispatch.
///
/// Passed by `&mut` reference to every command and event handler so they can
/// read/write state, access services, and submit new messages.
pub struct HandlerContext<'a, S, Sv> {
    /// Mutable application state.
    pub state: &'a mut S,
    /// Read-only services.
    pub services: &'a Sv,
    /// Buffered output for submitting new commands/events.
    pub out: &'a mut Out,
}

impl<'a, S, Sv> HandlerContext<'a, S, Sv> {
    /// Create a new handler context from its components.
    pub fn new(state: &'a mut S, services: &'a Sv, out: &'a mut Out) -> Self {
        Self {
            state,
            services,
            out,
        }
    }
}

/// Handler for a specific command type.
///
/// Implementations receive the concrete command and a [`HandlerContext`]
/// with mutable state, read-only services, and an output buffer.
///
/// Return [`CommandAction::Stop`] to prevent further handlers from seeing
/// this command. Return [`CommandAction::Continue`] to allow the next handler
/// to run.
pub trait CommandHandler<C, S, Sv>
where
    C: 'static,
{
    /// Handle a command.
    ///
    /// # Arguments
    ///
    /// * `cmd` - The concrete command to handle.
    /// * `ctx` - Handler context with state, services, and output buffer.
    ///
    /// # Returns
    ///
    /// [`CommandAction::Continue`] to allow further handlers,
    /// or [`CommandAction::Stop`] to halt propagation.
    fn handle(&self, cmd: &C, ctx: &mut HandlerContext<'_, S, Sv>) -> CommandAction;
}

/// Handler for a specific event type.
///
/// Implementations receive the concrete event and a [`HandlerContext`]
/// with mutable state, read-only services, and an output buffer. Events are
/// fire-and-forget — all registered handlers always run; there is no
/// interception.
pub trait EventHandler<E, S, Sv>
where
    E: 'static,
{
    /// Handle an event.
    ///
    /// # Arguments
    ///
    /// * `evt` - The concrete event to handle.
    /// * `ctx` - Handler context with state, services, and output buffer.
    fn handle(&self, evt: &E, ctx: &mut HandlerContext<'_, S, Sv>);
}
