//! Extension context for registering commands and subscribing to events.
//!
//! [`ExtensionContext`] is provided to extension methods. During [`activate`](crate::Extension::activate),
//! it accumulates registrations and subscriptions. After activation, [`ExtensionContext::take_registrations`]
//! flushes them to the host. In other callbacks, the context can send commands and events back.
//!
//! # Hosting modes
//!
//! [`ContextKind`] distinguishes between process-hosted extensions (communicating via stdio)
//! and in-memory extensions (running on OS threads). The [`ExtensionSink`] trait abstracts
//! how commands and events are sent — [`StdoutExtensionSink`] writes to stdout for process mode,
//! [`ChannelExtensionSink`] sends through a kanal channel for in-memory mode.

use std::sync::Arc;

use error_stack::Report;

use crate::codec::{CodecError, OutboundMessage, write_message};
use nullslop_core::{Command, Event};

/// Output from an extension to the host (command or event).
#[derive(Debug, Clone)]
pub enum ExtensionOutput {
    /// Extension sends a command to the host.
    Command(Command),
    /// Extension sends an event to the host.
    Event(Event),
}

/// Abstraction for sending commands and events from an extension to the host.
///
/// Implemented by [`StdoutExtensionSink`] (process mode) and
/// [`ChannelExtensionSink`] (in-memory mode).
pub trait ExtensionSink: Send + Sync + 'static {
    /// Sends a command to the host.
    ///
    /// # Errors
    ///
    /// Returns an error if the command cannot be delivered.
    fn send_command(&self, command: Command) -> Result<(), Report<CodecError>>;

    /// Sends an event to the host.
    ///
    /// # Errors
    ///
    /// Returns an error if the event cannot be delivered.
    fn send_event(&self, event: Event) -> Result<(), Report<CodecError>>;
}

/// Extension sink that writes to stdout (process-based extensions).
///
/// Serializes commands and events as JSON lines and writes them to stdout,
/// following the wire protocol.
pub struct StdoutExtensionSink;

impl ExtensionSink for StdoutExtensionSink {
    fn send_command(&self, command: Command) -> Result<(), Report<CodecError>> {
        write_message(&OutboundMessage::Command { command })
    }

    fn send_event(&self, event: Event) -> Result<(), Report<CodecError>> {
        write_message(&OutboundMessage::Event { event })
    }
}

/// Extension sink that sends through a kanal channel (in-memory extensions).
///
/// Avoids serialization overhead by passing [`ExtensionOutput`] directly through a channel.
pub struct ChannelExtensionSink {
    sender: kanal::Sender<ExtensionOutput>,
}

impl ChannelExtensionSink {
    /// Creates a new channel extension sink with the given sender.
    #[must_use]
    pub fn new(sender: kanal::Sender<ExtensionOutput>) -> Self {
        Self { sender }
    }
}

impl ExtensionSink for ChannelExtensionSink {
    fn send_command(&self, command: Command) -> Result<(), Report<CodecError>> {
        self.sender
            .send(ExtensionOutput::Command(command))
            .map_err(|_| Report::new(CodecError).attach("channel closed"))?;
        Ok(())
    }

    fn send_event(&self, event: Event) -> Result<(), Report<CodecError>> {
        self.sender
            .send(ExtensionOutput::Event(event))
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
/// the context can send commands and events back to the host via the [`ExtensionSink`].
pub struct ExtensionContext {
    commands: Vec<String>,
    subscriptions: Vec<String>,
    sink: Arc<dyn ExtensionSink>,
    kind: ContextKind,
    /// Host-assigned name for this extension instance.
    name: Option<String>,
}

impl ExtensionContext {
    /// Creates a new context with the given extension sink and hosting kind.
    ///
    /// Called by the [`run!`](crate::run!) macro (process mode) or the
    /// in-memory host — not typically used directly by extension authors.
    #[must_use]
    pub fn new(sink: Arc<dyn ExtensionSink>, kind: ContextKind) -> Self {
        Self {
            commands: Vec::new(),
            subscriptions: Vec::new(),
            sink,
            kind,
            name: None,
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
    /// Delegates to the underlying [`ExtensionSink`].
    ///
    /// # Errors
    ///
    /// Returns an error if the command cannot be delivered.
    pub fn send_command(&self, command: Command) -> Result<(), Report<CodecError>> {
        self.sink.send_command(command)
    }

    /// Sends an event to the host application.
    ///
    /// Delegates to the underlying [`ExtensionSink`].
    ///
    /// # Errors
    ///
    /// Returns an error if the event cannot be delivered.
    pub fn send_event(&self, event: Event) -> Result<(), Report<CodecError>> {
        self.sink.send_event(event)
    }

    /// Sets the extension's host-assigned name.
    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    /// Returns the extension's host-assigned name, if set.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
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

impl Default for ExtensionContext {
    fn default() -> Self {
        Self::new(Arc::new(StdoutExtensionSink), ContextKind::Process)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_command_accumulates() {
        // Given a new context.
        let mut ctx = ExtensionContext::default();

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
        let mut ctx = ExtensionContext::default();

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
        let mut ctx = ExtensionContext::default();
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
        let ctx = ExtensionContext::default();

        // Then kind is Process.
        assert!(matches!(ctx.kind(), ContextKind::Process));
    }

    #[test]
    fn channel_extension_sink_sends_command() {
        // Given a channel extension sink.
        let (tx, rx) = kanal::unbounded();
        let sink = ChannelExtensionSink::new(tx);

        // When sending a command.
        let cmd = nullslop_core::Command::AppQuit;
        sink.send_command(cmd).expect("send should succeed");

        // Then the command is received on the channel.
        let output = rx
            .try_recv()
            .expect("recv should succeed")
            .expect("should have value");
        assert!(matches!(
            output,
            ExtensionOutput::Command(nullslop_core::Command::AppQuit)
        ));
    }

    #[test]
    fn channel_extension_sink_command_error_on_closed_channel() {
        // Given a channel extension sink with a dropped receiver.
        let (tx, rx) = kanal::unbounded();
        drop(rx);
        let sink = ChannelExtensionSink::new(tx);

        // When sending a command.
        let result = sink.send_command(nullslop_core::Command::AppQuit);

        // Then it returns an error.
        assert!(result.is_err());
    }

    #[test]
    fn channel_extension_sink_sends_event() {
        // Given a channel extension sink.
        let (tx, rx) = kanal::unbounded();
        let sink = ChannelExtensionSink::new(tx);

        // When sending an event.
        let event = nullslop_core::Event::EventApplicationReady;
        sink.send_event(event).expect("send should succeed");

        // Then the event is received on the channel.
        let output = rx
            .try_recv()
            .expect("recv should succeed")
            .expect("should have value");
        assert!(matches!(
            output,
            ExtensionOutput::Event(nullslop_core::Event::EventApplicationReady)
        ));
    }

    #[test]
    fn channel_extension_sink_event_error_on_closed_channel() {
        // Given a channel extension sink with a dropped receiver.
        let (tx, rx) = kanal::unbounded();
        drop(rx);
        let sink = ChannelExtensionSink::new(tx);

        // When sending an event.
        let result = sink.send_event(nullslop_core::Event::EventApplicationReady);

        // Then it returns an error.
        assert!(result.is_err());
    }

    #[test]
    fn extension_context_send_event_delegates_to_sink() {
        // Given a context with a channel sink.
        let (tx, rx) = kanal::unbounded();
        let sink = Arc::new(ChannelExtensionSink::new(tx));
        let ctx = ExtensionContext::new(sink, ContextKind::Process);

        // When sending an event.
        ctx.send_event(nullslop_core::Event::EventApplicationReady)
            .expect("should succeed");

        // Then the event arrives on the channel.
        let output = rx
            .try_recv()
            .expect("recv should succeed")
            .expect("should have value");
        assert!(matches!(
            output,
            ExtensionOutput::Event(nullslop_core::Event::EventApplicationReady)
        ));
    }

    #[test]
    fn set_name_returns_name() {
        // Given a context.
        let mut ctx = ExtensionContext::default();

        // When no name is set.
        // Then name returns None.
        assert!(ctx.name().is_none());

        // When setting a name.
        ctx.set_name("test-ext".to_string());

        // Then name returns the set value.
        assert_eq!(ctx.name(), Some("test-ext"));
    }
}
