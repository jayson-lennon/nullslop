//! nullslop-echo: reference extension for nullslop.
//!
//! Implements [`Extension`] for both process and in-memory hosting.
//! Gets [`InMemoryExtension`] via blanket impl — no manual impl needed.
//! Echoes user messages as ALL CAPS extension entries after a 1-second delay.
//! Responds to lifecycle events (`ApplicationReady` → `ExtensionStarted`,
//! `ApplicationShuttingDown` → `ExtensionShutdownCompleted`).

use std::time::Duration;

use nullslop_core::{ChatEntryKind, Command, Event};
use nullslop_extension::{Extension, ExtensionContext};
use nullslop_protocol as npr;

/// Reference extension that echoes user messages back as extension entries.
pub struct EchoExtension;

impl Extension for EchoExtension {
    /// Registers the "echo" command and subscribes to events.
    fn activate(ctx: &mut ExtensionContext) -> Self {
        ctx.register_command("echo");
        ctx.subscribe("EventChatMessageSubmitted");
        ctx.subscribe("EventApplicationShuttingDown");
        ctx.subscribe("EventApplicationReady");
        Self
    }

    /// No-op — the interesting behavior is in [`Self::on_event`].
    fn on_command(&mut self, _command: &Command, _ctx: &ExtensionContext) {}

    /// Handles lifecycle events and echoes user messages.
    fn on_event(&mut self, event: &Event, ctx: &ExtensionContext) {
        match event {
            Event::EventApplicationShuttingDown => {
                if let Some(name) = ctx.name()
                    && let Err(e) = ctx.send_event(Event::EventExtensionShutdownCompleted {
                        payload: npr::event::ExtensionShutdownCompleted {
                            name: name.to_string(),
                        },
                    })
                {
                    tracing::error!(err = ?e, "echo extension failed to send shutdown completed");
                }
            }
            Event::EventApplicationReady => {
                if let Some(name) = ctx.name()
                    && let Err(e) = ctx.send_event(Event::EventExtensionStarted {
                        payload: npr::event::ExtensionStarted {
                            name: name.to_string(),
                        },
                    })
                {
                    tracing::error!(err = ?e, "echo extension failed to send started");
                }
            }
            _ => send_echo(event, ctx),
        }
    }

    /// No cleanup needed on deactivation.
    fn deactivate(&mut self) {}
}

/// Shared echo logic: sleep 1s, then send ALL CAPS echo command.
fn send_echo(event: &Event, ctx: &ExtensionContext) {
    if let Event::EventChatMessageSubmitted {
        payload: npr::event::EventChatMessageSubmitted { entry },
    } = event
        && let ChatEntryKind::User(text) = &entry.kind
    {
        std::thread::sleep(Duration::from_secs(1));
        if let Err(e) = ctx.send_command(Command::CustomCommand {
            payload: npr::command::CustomCommand {
                name: "echo".to_string(),
                args: serde_json::json!({
                    "source": "nullslop-echo",
                    "text": text.to_uppercase(),
                }),
            },
        }) {
            tracing::error!(err = ?e, "echo extension failed to send command");
        }
    }
}
