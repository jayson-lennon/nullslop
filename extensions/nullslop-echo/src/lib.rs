//! nullslop-echo: reference extension for nullslop.
//!
//! Implements [`Extension`] for both process and in-memory hosting.
//! Gets [`InMemoryExtension`] via blanket impl — no manual impl needed.
//! Echoes user messages as ALL CAPS extension entries after a 1-second delay.

use std::time::Duration;

use nullslop_core::{ChatEntryKind, Command, Event};
use nullslop_extension::{Context, Extension};
use nullslop_protocol as npr;

/// Reference extension that echoes user messages back as extension entries.
pub struct EchoExtension;

impl Extension for EchoExtension {
    /// Registers the "echo" command and subscribes to `EventChatMessageSubmitted` events.
    fn activate(ctx: &mut Context) -> Self {
        ctx.register_command("echo");
        ctx.subscribe("EventChatMessageSubmitted");
        Self
    }

    /// No-op — the interesting behavior is in [`Self::on_event`].
    fn on_command(&mut self, _command: &Command, _ctx: &Context) {}

    /// Echoes user messages back as extension entries via the `echo` command.
    fn on_event(&mut self, event: &Event, ctx: &Context) {
        send_echo(event, ctx);
    }

    /// No cleanup needed on deactivation.
    fn deactivate(&mut self) {}
}

/// Shared echo logic: sleep 1s, then send ALL CAPS echo command.
fn send_echo(event: &Event, ctx: &Context) {
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
