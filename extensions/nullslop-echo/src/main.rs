//! nullslop-echo: reference extension for nullslop.
//!
//! Proves the full round-trip: user submits chat → host broadcasts `NewChatEntry`
//! → echo extension receives event → echo sends `echo` command → host adds
//! system chat entry.

use nullslop_core::{ChatEntryKind, Command, Event};
use nullslop_extension::{Context, Extension, run};

/// Reference extension that echoes user messages back as system messages.
struct EchoExtension;

impl Extension for EchoExtension {
    /// Registers the "echo" command and subscribes to `NewChatEntry` events.
    fn activate(ctx: &mut Context) -> Self {
        ctx.register_command("echo");
        ctx.subscribe("NewChatEntry");
        Self
    }

    /// No-op — the interesting behavior is in [`Self::on_event`].
    fn on_command(&mut self, _command: &Command, _ctx: &Context) {}

    /// Echoes user messages back as system messages via the `echo` command.
    fn on_event(&mut self, event: &Event, ctx: &Context) {
        if let Event::NewChatEntry { entry } = event
            && let ChatEntryKind::User(text) = &entry.kind
            && let Err(e) = ctx.send_command(Command::Custom {
                name: "echo".to_string(),
                args: serde_json::json!({ "text": format!("echo: {text}") }),
            })
        {
            tracing::error!(err = ?e, "echo extension failed to send command");
        }
    }

    /// No cleanup needed on deactivation.
    fn deactivate(&mut self) {}
}

run!(EchoExtension);
