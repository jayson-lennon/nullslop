//! nullslop-echo: reference extension for nullslop.
//!
//! Proves the full round-trip: user submits chat → host broadcasts `EventChatMessageSubmitted`
//! → echo extension receives event → echo sends `CustomCommand` → host adds
//! system chat entry.

use npr::event::EventChatMessageSubmitted;
use nullslop_core::{ChatEntryKind, Command, Event};
use nullslop_extension::{Context, Extension, run};
use nullslop_protocol as npr;

/// Reference extension that echoes user messages back as system messages.
struct EchoExtension;

impl Extension for EchoExtension {
    /// Registers the "echo" command and subscribes to `EventChatMessageSubmitted` events.
    fn activate(ctx: &mut Context) -> Self {
        ctx.register_command("echo");
        ctx.subscribe("EventChatMessageSubmitted");
        Self
    }

    /// No-op — the interesting behavior is in [`Self::on_event`].
    fn on_command(&mut self, _command: &Command, _ctx: &Context) {}

    /// Echoes user messages back as system messages via the `echo` command.
    fn on_event(&mut self, event: &Event, ctx: &Context) {
        if let Event::EventChatMessageSubmitted {
            payload: EventChatMessageSubmitted { entry },
        } = event
            && let ChatEntryKind::User(text) = &entry.kind
            && let Err(e) = ctx.send_command(Command::CustomCommand {
                payload: npr::command::CustomCommand {
                    name: "echo".to_string(),
                    args: serde_json::json!({ "text": format!("echo: {text}") }),
                },
            })
        {
            tracing::error!(err = ?e, "echo extension failed to send command");
        }
    }

    /// No cleanup needed on deactivation.
    fn deactivate(&mut self) {}
}

run!(EchoExtension);
