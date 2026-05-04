//! Message queue — enqueues, dispatches, and drains user messages to the LLM.
//!
//! When a user submits a message, it goes through this handler. If the session
//! is idle, the message is dispatched immediately to the LLM. If the session
//! is busy (sending or streaming), the message is enqueued for later dispatch.
//!
//! On normal stream completion, all queued messages are dispatched at once in a single LLM call.
//! On cancel, the queue is drained and all messages are concatenated back into
//! the input box so the user doesn't lose their text.

use crate::AppState;
use npr::chat_input::{EnqueueUserMessage, SetChatInputText};
use npr::provider::{
    CancelStream, SendToLlmProvider, StreamCompleted, StreamCompletedReason, entries_to_messages,
};
use nullslop_component_core::{HandlerContext, define_handler};
use nullslop_protocol as npr;
use nullslop_protocol::CommandAction;
use nullslop_services::Services;

define_handler! {
    pub(crate) struct MessageQueueHandler;

    commands {
        EnqueueUserMessage: on_enqueue_user_message,
        CancelStream: on_cancel_stream,
        SetChatInputText: on_set_chat_input_text,
    }

    events {
        StreamCompleted: on_stream_completed,
    }
}

impl MessageQueueHandler {
    /// Enqueues a user message, dispatching immediately if idle or queuing if busy.
    fn on_enqueue_user_message(
        cmd: &EnqueueUserMessage,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        let session = ctx.state.session_mut(&cmd.session_id);

        if session.is_idle() {
            // Dispatch immediately: push to history, convert, send to LLM.
            let entry = npr::ChatEntry::user(&cmd.text);
            session.push_entry(entry);
            let history = session.history();
            let messages = entries_to_messages(history);
            session.begin_sending();

            ctx.out.submit_command(npr::Command::SendToLlmProvider {
                payload: SendToLlmProvider {
                    session_id: cmd.session_id.clone(),
                    messages,
                    provider_id: None,
                },
            });
        } else {
            // Session is busy — enqueue for later.
            session.enqueue_message(cmd.text.clone());
        }

        CommandAction::Continue
    }

    /// Cancels the active stream and restores queued messages to the input box.
    fn on_cancel_stream(
        cmd: &CancelStream,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        let session = ctx.state.session_mut(&cmd.session_id);
        session.cancel_streaming();

        let drained: Vec<String> = session.drain_queue().into_iter().collect();
        if !drained.is_empty() {
            let restored = drained.join("\n");
            ctx.out.submit_command(npr::Command::SetChatInputText {
                payload: SetChatInputText {
                    session_id: cmd.session_id.clone(),
                    text: restored,
                },
            });
        }

        CommandAction::Continue
    }

    /// Replaces the chat input text for a session.
    fn on_set_chat_input_text(
        cmd: &SetChatInputText,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        let session = ctx.state.session_mut(&cmd.session_id);
        session.chat_input_mut().replace_all(cmd.text.clone());
        CommandAction::Continue
    }

    /// Handles stream completion, dispatching all queued messages at once if any.
    fn on_stream_completed(
        evt: &StreamCompleted,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) {
        let session = ctx.state.session_mut(&evt.session_id);
        // finish_streaming clears both is_streaming and is_sending.
        session.finish_streaming();

        // Only dispatch on normal completion.
        // On cancel, the queue was already drained by on_cancel_stream.
        if evt.reason != StreamCompletedReason::Finished {
            return;
        }

        let drained: Vec<String> = session.drain_queue().into_iter().collect();
        if drained.is_empty() {
            return;
        }

        // Push all queued messages as individual user entries.
        for text in &drained {
            session.push_entry(npr::ChatEntry::user(text));
        }

        let history = session.history();
        let messages = entries_to_messages(history);
        session.begin_sending();

        ctx.out.submit_command(npr::Command::SendToLlmProvider {
            payload: SendToLlmProvider {
                session_id: evt.session_id.clone(),
                messages,
                provider_id: None,
            },
        });
    }
}



