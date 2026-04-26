//! The extension trait for building nullslop extensions.
//!
//! Extension authors implement [`Extension`] and use the [`run!`](crate::run!)
//! macro to generate their binary's `main()` function.

use nullslop_core::{Command, Event};

use crate::Context;

/// Trait for implementing a nullslop extension.
///
/// Extensions are activated when the host sends an `initialize` message.
/// They receive commands they've registered for and events they've subscribed to.
/// When the host sends `shutdown`, the extension should clean up and exit.
pub trait Extension {
    /// Activates the extension. Use `ctx` to register commands and subscribe to events.
    ///
    /// This is an associated function (not a method) — it returns `Self`,
    /// constructing the extension during activation.
    fn activate(ctx: &mut Context) -> Self;

    /// Handles a command dispatched to this extension.
    ///
    /// Errors are logged, not propagated across the process boundary.
    fn on_command(&mut self, command: &Command, ctx: &Context);

    /// Handles an event this extension subscribed to.
    ///
    /// Errors are logged, not propagated across the process boundary.
    fn on_event(&mut self, event: &Event, ctx: &Context);

    /// Deactivates the extension. Called before the process exits.
    fn deactivate(&mut self);
}
