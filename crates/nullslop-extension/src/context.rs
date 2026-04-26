//! Extension context for registering commands and subscribing to events.
//!
//! [`Context`] is provided to extension methods. During [`activate`](crate::Extension::activate),
//! it accumulates registrations and subscriptions. After activation, [`Context::take_registrations`]
//! flushes them to the host. In other callbacks, the context can send commands back.

use error_stack::Report;

use crate::codec::{CodecError, OutboundMessage, write_message};
use nullslop_core::Command;

/// Context provided to extension methods.
///
/// Accumulates registrations and subscriptions during `activate`,
/// then flushes them to the host. During `on_command`/`on_event`,
/// the context can send commands back to the host.
pub struct Context {
    commands: Vec<String>,
    subscriptions: Vec<String>,
}

impl Context {
    /// Creates a new empty context.
    ///
    /// Called by the [`run!`](crate::run!) macro — not typically used directly.
    #[must_use]
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            subscriptions: Vec::new(),
        }
    }

    /// Registers a command that this extension handles.
    pub fn register_command(&mut self, name: &str) {
        self.commands.push(name.to_string());
    }

    /// Subscribes to an event by name.
    pub fn subscribe(&mut self, event: &str) {
        self.subscriptions.push(event.to_string());
    }

    /// Sends a command to the host application.
    ///
    /// # Errors
    ///
    /// Returns an error if the message cannot be serialized or written.
    pub fn send_command(&self, command: Command) -> Result<(), Report<CodecError>> {
        write_message(&OutboundMessage::Command { command })
    }

    /// Returns the accumulated registrations and subscriptions, clearing them.
    ///
    /// Called by the [`run!`](crate::run!) macro after `activate`.
    pub fn take_registrations(&mut self) -> (Vec<String>, Vec<String>) {
        let commands = std::mem::take(&mut self.commands);
        let subscriptions = std::mem::take(&mut self.subscriptions);
        (commands, subscriptions)
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_command_accumulates() {
        // Given a new context.
        let mut ctx = Context::new();

        // When registering two commands.
        ctx.register_command("echo");
        ctx.register_command("reverse");

        // Then take_registrations returns both.
        let (commands, _) = ctx.take_registrations();
        assert_eq!(commands, vec!["echo", "reverse"]);
    }

    #[test]
    fn subscribe_accumulates() {
        // Given a new context.
        let mut ctx = Context::new();

        // When subscribing to events.
        ctx.subscribe("NewChatEntry");
        ctx.subscribe("ApplicationReady");

        // Then take_registrations returns both subscriptions.
        let (_, subscriptions) = ctx.take_registrations();
        assert_eq!(subscriptions, vec!["NewChatEntry", "ApplicationReady"]);
    }

    #[test]
    fn take_registrations_clears() {
        // Given a context with registrations.
        let mut ctx = Context::new();
        ctx.register_command("echo");
        ctx.subscribe("NewChatEntry");

        // When calling take_registrations twice.
        let first = ctx.take_registrations();
        let second = ctx.take_registrations();

        // Then first has data and second is empty.
        assert!(!first.0.is_empty());
        assert!(!first.1.is_empty());
        assert!(second.0.is_empty());
        assert!(second.1.is_empty());
    }
}
