//! Extension context for registering commands and subscribing to events.
//!
//! [`Context`] is provided to extension methods. During [`activate`](crate::Extension::activate),
//! it accumulates registrations and subscriptions. After activation, [`Context::take_registrations`]
//! flushes them to the host. In other callbacks, the context can send commands back.
//!
//! # Hosting modes
//!
//! [`ContextKind`] distinguishes between process-hosted extensions (communicating via stdio)
//! and in-memory extensions (running on OS threads). The [`CommandSink`] trait abstracts
//! how commands are sent — [`StdoutCommandSink`] writes to stdout for process mode,
//! [`ChannelCommandSink`] sends through a kanal channel for in-memory mode.

use std::sync::Arc;

use error_stack::Report;

use crate::codec::{CodecError, OutboundMessage, write_message};
use nullslop_core::Command;

/// Abstraction for sending commands from an extension to the host.
///
/// Implemented by [`StdoutCommandSink`] (process mode) and
/// [`ChannelCommandSink`] (in-memory mode).
pub trait CommandSink: Send + Sync + 'static {
    /// Sends a command to the host.
    ///
    /// # Errors
    ///
    /// Returns an error if the command cannot be delivered.
    fn send_command(&self, command: Command) -> Result<(), Report<CodecError>>;
}

/// Command sink that writes to stdout (process-based extensions).
///
/// Serializes the command as a JSON line and writes it to stdout,
/// following the wire protocol.
pub struct StdoutCommandSink;

impl CommandSink for StdoutCommandSink {
    fn send_command(&self, command: Command) -> Result<(), Report<CodecError>> {
        write_message(&OutboundMessage::Command { command })
    }
}

/// Command sink that sends commands through a kanal channel (in-memory extensions).
///
/// Avoids serialization overhead by passing commands directly through a channel.
pub struct ChannelCommandSink {
    sender: kanal::Sender<Command>,
}

impl ChannelCommandSink {
    /// Creates a new channel command sink with the given sender.
    #[must_use]
    pub fn new(sender: kanal::Sender<Command>) -> Self {
        Self { sender }
    }
}

impl CommandSink for ChannelCommandSink {
    fn send_command(&self, command: Command) -> Result<(), Report<CodecError>> {
        self.sender
            .send(command)
            .map_err(|_| Report::new(CodecError).attach("channel closed"))?;
        Ok(())
    }
}

/// How the extension is hosted.
pub enum ContextKind {
    /// Running as a child process communicating via stdio.
    Process,
    /// Running in-memory with access to the tokio runtime for spawning async tasks.
    InMemory {
        /// Handle to the tokio runtime.
        handle: tokio::runtime::Handle,
    },
}

/// Context provided to extension methods.
///
/// Accumulates registrations and subscriptions during `activate`,
/// then flushes them to the host. During `on_command`/`on_event`,
/// the context can send commands back to the host via the [`CommandSink`].
pub struct Context {
    commands: Vec<String>,
    subscriptions: Vec<String>,
    sink: Arc<dyn CommandSink>,
    kind: ContextKind,
}

impl Context {
    /// Creates a new context with the given command sink and hosting kind.
    ///
    /// Called by the [`run!`](crate::run!) macro (process mode) or the
    /// in-memory host — not typically used directly by extension authors.
    #[must_use]
    pub fn new(sink: Arc<dyn CommandSink>, kind: ContextKind) -> Self {
        Self {
            commands: Vec::new(),
            subscriptions: Vec::new(),
            sink,
            kind,
        }
    }

    /// Returns the hosting context kind.
    #[must_use]
    pub fn kind(&self) -> &ContextKind {
        &self.kind
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
    /// Delegates to the underlying [`CommandSink`].
    ///
    /// # Errors
    ///
    /// Returns an error if the command cannot be delivered.
    pub fn send_command(&self, command: Command) -> Result<(), Report<CodecError>> {
        self.sink.send_command(command)
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
        Self::new(
            Arc::new(StdoutCommandSink),
            ContextKind::Process,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_command_accumulates() {
        // Given a new context.
        let mut ctx = Context::default();

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
        let mut ctx = Context::default();

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
        let mut ctx = Context::default();
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

    #[test]
    fn default_context_is_process_mode() {
        // Given a default context.
        let ctx = Context::default();

        // Then kind is Process.
        assert!(matches!(ctx.kind(), ContextKind::Process));
    }

    #[test]
    fn channel_command_sink_sends_command() {
        // Given a channel command sink.
        let (tx, rx) = kanal::unbounded();
        let sink = ChannelCommandSink::new(tx);

        // When sending a command.
        let cmd = nullslop_core::Command::AppQuit;
        sink.send_command(cmd).expect("send should succeed");

        // Then the command is received on the channel.
        assert!(matches!(rx.try_recv(), Ok(Some(nullslop_core::Command::AppQuit))));
    }

    #[test]
    fn channel_command_sink_returns_error_on_closed_channel() {
        // Given a channel command sink with a dropped receiver.
        let (tx, rx) = kanal::unbounded();
        drop(rx);
        let sink = ChannelCommandSink::new(tx);

        // When sending a command.
        let result = sink.send_command(nullslop_core::Command::AppQuit);

        // Then it returns an error.
        assert!(result.is_err());
    }
}
