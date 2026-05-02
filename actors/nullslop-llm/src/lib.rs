//! nullslop-llm: LLM streaming actor for nullslop.
//!
//! Subscribes to `SendToLlmProvider` commands (full conversation context)
//! and `CancelStream` commands. On send, creates an LLM service via the
//! factory, streams tokens back as `StreamToken` commands, and emits
//! `StreamCompleted` when done. On cancel, aborts the active stream task.

use std::collections::HashMap;

use futures::StreamExt as _;
use nullslop_actor::{Actor, ActorContext, ActorEnvelope, SystemMessage};
use nullslop_protocol::chat_input::PushChatEntry;
use nullslop_protocol::provider::LlmMessage;
use nullslop_protocol::provider::{
    CancelStream, SendToLlmProvider, StreamCompleted, StreamCompletedReason, StreamToken,
};
use nullslop_protocol::{ChatEntry, Command, Event, SessionId};
use nullslop_services::providers::LlmServiceFactoryService;
use nullslop_services::providers::llm_messages_to_chat_messages;

/// Direct message type for the LLM actor.
///
/// Currently unused — the actor responds to bus commands.
/// Reserved for future intra-actor communication.
pub enum LlmDirectMsg {}

/// LLM streaming actor.
///
/// Holds a reference to the LLM service factory and tracks active
/// streaming tasks per session.
pub struct LlmActor {
    /// Factory for creating LLM service instances.
    factory: LlmServiceFactoryService,
    /// Active stream tasks, keyed by session ID.
    tasks: HashMap<SessionId, tokio::task::JoinHandle<()>>,
}

impl Actor for LlmActor {
    type Message = LlmDirectMsg;

    #[expect(
        clippy::expect_used,
        reason = "data is injected by the host before activate is called"
    )]
    fn activate(ctx: &mut ActorContext) -> Self {
        ctx.subscribe_command::<SendToLlmProvider>();
        ctx.subscribe_command::<CancelStream>();

        let factory = ctx
            .take_data::<LlmServiceFactoryService>()
            .expect("LlmServiceFactoryService must be injected via ctx.set_data() before activate");

        Self {
            factory,
            tasks: HashMap::new(),
        }
    }

    async fn handle(&mut self, msg: ActorEnvelope<LlmDirectMsg>, ctx: &ActorContext) {
        match msg {
            ActorEnvelope::Command(command) => self.handle_command(&command, ctx),
            ActorEnvelope::System(SystemMessage::ApplicationShuttingDown) => {
                self.cancel_all();
                ctx.announce_shutdown_completed();
            }
            ActorEnvelope::System(SystemMessage::ApplicationReady) => {
                ctx.announce_started();
            }
            ActorEnvelope::Event(_) | ActorEnvelope::Direct(_) | ActorEnvelope::Shutdown => {}
        }
    }

    async fn shutdown(self) {
        self.cancel_all();
    }
}

impl LlmActor {
    /// Dispatches incoming commands to the appropriate handler.
    fn handle_command(&mut self, command: &Command, ctx: &ActorContext) {
        match command {
            Command::SendToLlmProvider { payload } => {
                self.start_stream(payload.session_id.clone(), payload.messages.clone(), ctx);
            }
            Command::CancelStream { payload } => {
                self.cancel_stream(&payload.session_id, ctx);
            }
            _ => {}
        }
    }

    /// Starts an LLM streaming response for a session, aborting any existing stream.
    fn start_stream(
        &mut self,
        session_id: SessionId,
        messages: Vec<LlmMessage>,
        ctx: &ActorContext,
    ) {
        // Abort any existing stream for this session.
        if let Some(handle) = self.tasks.remove(&session_id) {
            handle.abort();
        }

        let factory = self.factory.clone();
        let sink = ctx.sink();
        let session_id_clone = session_id.clone();

        let handle = tokio::spawn(async move {
            // Convert protocol messages to llm crate messages.
            let chat_messages = llm_messages_to_chat_messages(&messages);

            let service = match factory.create() {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!(err = ?e, "failed to create LLM service");
                    let _ = sink.send_command(Command::PushChatEntry {
                        payload: PushChatEntry {
                            session_id: session_id_clone.clone(),
                            entry: ChatEntry::system("LLM service creation failed"),
                        },
                    });
                    let _ = sink.send_event(Event::StreamCompleted {
                        payload: StreamCompleted {
                            session_id: session_id_clone,
                            reason: StreamCompletedReason::Finished,
                        },
                    });
                    return;
                }
            };

            let stream = match service.chat_stream(chat_messages).await {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!(err = ?e, "failed to start LLM stream");
                    let _ = sink.send_command(Command::PushChatEntry {
                        payload: PushChatEntry {
                            session_id: session_id_clone.clone(),
                            entry: ChatEntry::system(format!("LLM stream error: {e:?}")),
                        },
                    });
                    let _ = sink.send_event(Event::StreamCompleted {
                        payload: StreamCompleted {
                            session_id: session_id_clone,
                            reason: StreamCompletedReason::Finished,
                        },
                    });
                    return;
                }
            };

            let mut index = 0usize;
            let mut stream = std::pin::pin!(stream);
            while let Some(result) = stream.next().await {
                match result {
                    Ok(token) => {
                        let _ = sink.send_command(Command::StreamToken {
                            payload: StreamToken {
                                session_id: session_id_clone.clone(),
                                index,
                                token,
                            },
                        });
                        index += 1;
                    }
                    Err(e) => {
                        tracing::error!(err = ?e, "LLM stream token error");
                        break;
                    }
                }
            }

            let _ = sink.send_event(Event::StreamCompleted {
                payload: StreamCompleted {
                    session_id: session_id_clone,
                    reason: StreamCompletedReason::Finished,
                },
            });
        });

        self.tasks.insert(session_id, handle);
    }

    /// Cancels the active stream for a session and emits a completion event.
    fn cancel_stream(&mut self, session_id: &SessionId, ctx: &ActorContext) {
        if let Some(handle) = self.tasks.remove(session_id) {
            handle.abort();
            let _ = ctx.send_event(Event::StreamCompleted {
                payload: StreamCompleted {
                    session_id: session_id.clone(),
                    reason: StreamCompletedReason::Canceled,
                },
            });
        }
    }

    /// Cancels all active streams across all sessions.
    fn cancel_all(&self) {
        for handle in self.tasks.values() {
            handle.abort();
        }
    }
}
