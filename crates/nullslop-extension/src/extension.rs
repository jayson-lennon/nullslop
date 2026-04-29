//! The extension trait for building nullslop extensions.
//!
//! Extension authors implement [`Extension`] and use the [`run!`](crate::run!)
//! macro to generate their binary's `main()` function.
//!
//! For in-memory hosting, [`InMemoryExtension`] is automatically provided via
//! a blanket impl for all types that implement [`Extension`] + [`Send`].
//! Extension authors never need to implement [`InMemoryExtension`] directly.

use nullslop_core::{Command, Event};

use crate::ExtensionContext;

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
    fn activate(ctx: &mut ExtensionContext) -> Self;

    /// Handles a command dispatched to this extension.
    ///
    /// Errors are logged, not propagated across the process boundary.
    fn on_command(&mut self, command: &Command, ctx: &ExtensionContext);

    /// Handles an event this extension subscribed to.
    ///
    /// Errors are logged, not propagated across the process boundary.
    fn on_event(&mut self, event: &Event, ctx: &ExtensionContext);

    /// Deactivates the extension. Called before the process exits.
    fn deactivate(&mut self);
}

/// Object-safe extension trait for in-memory hosting.
///
/// Automatically implemented for all types that implement [`Extension`]
/// and are [`Send`]. Extension authors only implement [`Extension`];
/// this trait is provided by the blanket impl.
///
/// The host creates `Box<dyn InMemoryExtension>` and calls [`activate`](Self::activate)
/// with `&mut self`, which replaces the dummy instance with the properly-activated one
/// via `*self = E::activate(ctx)`.
pub trait InMemoryExtension: Send + 'static {
    /// Activates the extension.
    fn activate(&mut self, ctx: &mut ExtensionContext);
    /// Handles a command dispatched to this extension.
    fn on_command(&mut self, command: &Command, ctx: &ExtensionContext);
    /// Handles an event this extension subscribed to.
    fn on_event(&mut self, event: &Event, ctx: &ExtensionContext);
    /// Deactivates the extension.
    fn deactivate(&mut self);
}

impl<E: Extension + Send + 'static> InMemoryExtension for E {
    fn activate(&mut self, ctx: &mut ExtensionContext) {
        // Replace dummy self with properly-activated instance.
        *self = E::activate(ctx);
    }

    fn on_command(&mut self, command: &Command, ctx: &ExtensionContext) {
        Extension::on_command(self, command, ctx);
    }

    fn on_event(&mut self, event: &Event, ctx: &ExtensionContext) {
        Extension::on_event(self, event, ctx);
    }

    fn deactivate(&mut self) {
        Extension::deactivate(self);
    }
}
