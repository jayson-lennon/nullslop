//! nullslop-echo: reference actor for nullslop.
//!
//! Implements [`Actor`] for in-memory hosting. Echoes user messages as ALL CAPS
//! actor entries after a 1-second delay. Lifecycle announcements
//! (`ActorStarted`, `ActorShutdownCompleted`) are sent via the `ActorContext`
//! helpers, which are automatically triggered by host-broadcast lifecycle events.

use std::time::Duration;

use nullslop_actor::{Actor, ActorContext, ActorEnvelope};
use nullslop_protocol::chat_input::ChatEntrySubmitted;
use nullslop_protocol::{ChatEntryKind, Command, Event};

/// Direct message type for the echo actor.
/// Currently unused — the echo actor only responds to bus events.
pub enum EchoDirectMsg {}

/// Reference echo actor that echoes user messages back as actor entries.
pub struct EchoActor;

impl Actor for EchoActor {
    type Message = EchoDirectMsg;

    fn activate(ctx: &mut ActorContext) -> Self {
        ctx.subscribe_event::<ChatEntrySubmitted>();

        Self
    }

    async fn handle(&mut self, msg: ActorEnvelope<EchoDirectMsg>, ctx: &ActorContext) {
        match msg {
            ActorEnvelope::Event(event) => self.on_event(&event, ctx).await,
            ActorEnvelope::Command(_) | ActorEnvelope::Direct(_) | ActorEnvelope::Shutdown => {}
        }
    }

    async fn shutdown(self) {}
}

impl EchoActor {
    async fn on_event(&self, event: &Event, ctx: &ActorContext) {
        match event {
            Event::ApplicationShuttingDown => {
                ctx.announce_shutdown_completed();
            }
            Event::ApplicationReady => {
                ctx.announce_started();
            }
            _ => Self::send_echo(event, ctx).await,
        }
    }

    async fn send_echo(event: &Event, ctx: &ActorContext) {
        if let Event::ChatEntrySubmitted { payload } = event
            && let ChatEntryKind::User(text) = &payload.entry.kind
        {
            tokio::time::sleep(Duration::from_secs(1)).await;
            if let Err(e) = ctx.send_command(Command::PushChatEntry {
                payload: nullslop_protocol::chat_input::PushChatEntry {
                    entry: nullslop_protocol::ChatEntry::actor(
                        "nullslop-echo",
                        text.to_uppercase(),
                    ),
                },
            }) {
                tracing::error!(err = ?e, "echo actor failed to send command");
            }
        }
    }
}
