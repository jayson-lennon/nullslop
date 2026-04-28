//! Handler traits for commands and events.
//!
//! [`CommandHandler<C>`] and [`EventHandler<E>`] are generic traits that plugins
//! implement for specific command or event types. The [`Bus`](crate::Bus) dispatches
//! to handlers via [`TypeId`], so each handler receives the concrete type.

use nullslop_protocol::{AppState, CommandAction};

use crate::Out;

/// Handler for a specific command type `C`.
///
/// Implementations receive a concrete command reference and can inspect state
/// and submit new commands/events via `out`.
///
/// Return [`CommandAction::Stop`] to prevent further handlers from seeing this command.
/// Return [`CommandAction::Continue`] to allow the next handler to run.
///
/// # Type parameter
///
/// `C` must be `'static` so the bus can use [`TypeId::of::<C>()`](TypeId::of)
/// for dispatch.
pub trait CommandHandler<C: 'static> {
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
    fn handle(&self, cmd: &C, state: &mut AppState, out: &mut Out) -> CommandAction;
}

/// Handler for a specific event type `E`.
///
/// Implementations receive a concrete event reference and can inspect state
/// and submit new commands/events via `out`. Events are fire-and-forget —
/// all registered handlers always run; there is no interception.
///
/// # Type parameter
///
/// `E` must be `'static` so the bus can use [`TypeId::of::<E>()`](TypeId::of)
/// for dispatch.
pub trait EventHandler<E: 'static> {
    /// Handle an event.
    ///
    /// # Arguments
    ///
    /// * `evt` - The concrete event to handle.
    /// * `state` - Mutable application state.
    /// * `out` - Buffered output for submitting new commands/events.
    fn handle(&self, evt: &E, state: &mut AppState, out: &mut Out);
}
