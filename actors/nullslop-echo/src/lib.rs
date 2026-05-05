//! nullslop-echo: reference actor for nullslop.
//!
//! Implements [`Actor`] for in-memory hosting. Echoes user messages as ALL CAPS
//! actor entries after a 1-second delay. Lifecycle announcements
//! (`ActorStarted`, `ActorShutdownCompleted`) are sent via the `ActorContext`
//! helpers, which are automatically triggered by host-broadcast lifecycle events.

use std::time::Duration;

use nullslop_actor::{Actor, ActorContext, ActorEnvelope, SystemMessage};
use nullslop_protocol::chat_input::{self, ChatEntrySubmitted};
use nullslop_protocol::{ChatEntry, ChatEntryKind, Command, Event};

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
            ActorEnvelope::System(SystemMessage::ApplicationShuttingDown) => {
                ctx.announce_shutdown_completed();
            }
            ActorEnvelope::System(SystemMessage::ApplicationReady) => {
                ctx.announce_started();
            }
            ActorEnvelope::Event(event) => Self::process_event(&event, ctx).await,
            ActorEnvelope::Command(_) | ActorEnvelope::Direct(_) | ActorEnvelope::Shutdown => {}
        }
    }

    async fn shutdown(self) {}
}

impl EchoActor {
    /// Processes an incoming event, echoing user messages as ALL CAPS actor entries.
    async fn process_event(event: &Event, ctx: &ActorContext) {
        match event {
            Event::ChatEntrySubmitted {
                payload:
                    ChatEntrySubmitted {
                        session_id,
                        entry:
                            ChatEntry {
                                kind: ChatEntryKind::User(text),
                                ..
                            },
                        ..
                    },
            } => {
                tokio::time::sleep(Duration::from_secs(1)).await;
                if let Err(e) = ctx.send_command(Command::PushChatEntry {
                    payload: chat_input::PushChatEntry {
                        session_id: session_id.clone(),
                        entry: ChatEntry::actor("echo", text.to_uppercase()),
                    },
                }) {
                    tracing::error!(err = ?e, "echo actor failed to send command");
                }
            }
            _ => {}
        }
    }
}
