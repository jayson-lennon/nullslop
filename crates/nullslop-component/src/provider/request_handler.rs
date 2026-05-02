//! Message queue — enqueues, dispatches, and drains user messages to the LLM.
//!
//! When a user submits a message, it goes through this handler. If the session
//! is idle, the message is dispatched immediately to the LLM. If the session
//! is busy (sending or streaming), the message is enqueued for later dispatch.
//!
//! On normal stream completion, the next queued message is dispatched automatically.
//! On cancel, the queue is drained and all messages are concatenated back into
//! the input box so the user doesn't lose their text.

use crate::AppState;
use npr::chat_input::{EnqueueUserMessage, SetChatInputText};
use npr::provider::{
    CancelStream, SendToLlmProvider, StreamCompleted, StreamCompletedReason, entries_to_messages,
};
use nullslop_component_core::{Out, define_handler};
use nullslop_protocol as npr;
use nullslop_protocol::CommandAction;

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
        state: &mut AppState,
        out: &mut Out,
    ) -> CommandAction {
        let session = state.session_mut(&cmd.session_id);

        if session.is_idle() {
            // Dispatch immediately: push to history, convert, send to LLM.
            let entry = npr::ChatEntry::user(&cmd.text);
            session.push_entry(entry);
            let history = session.history();
            let messages = entries_to_messages(history);
            session.begin_sending();

            out.submit_command(npr::Command::SendToLlmProvider {
                payload: SendToLlmProvider {
                    session_id: cmd.session_id.clone(),
                    messages,
                },
            });
        } else {
            // Session is busy — enqueue for later.
            session.enqueue_message(cmd.text.clone());
        }

        CommandAction::Continue
    }

    /// Cancels the active stream and restores queued messages to the input box.
    fn on_cancel_stream(cmd: &CancelStream, state: &mut AppState, out: &mut Out) -> CommandAction {
        let session = state.session_mut(&cmd.session_id);
        session.cancel_streaming();

        let drained: Vec<String> = session.drain_queue().into_iter().collect();
        if !drained.is_empty() {
            let restored = drained.join("\n");
            out.submit_command(npr::Command::SetChatInputText {
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
        state: &mut AppState,
        _out: &mut Out,
    ) -> CommandAction {
        let session = state.session_mut(&cmd.session_id);
        session.chat_input_mut().replace_all(cmd.text.clone());
        CommandAction::Continue
    }

    /// Handles stream completion, dispatching the next queued message if available.
    fn on_stream_completed(evt: &StreamCompleted, state: &mut AppState, out: &mut Out) {
        let session = state.session_mut(&evt.session_id);
        // finish_streaming clears both is_streaming and is_sending.
        session.finish_streaming();

        // Only dispatch next on normal completion.
        // On cancel, the queue was already drained by on_cancel_stream.
        if evt.reason == StreamCompletedReason::Finished
            && let Some(text) = session.dequeue_message()
        {
            let entry = npr::ChatEntry::user(&text);
            session.push_entry(entry);
            let history = session.history();
            let messages = entries_to_messages(history);
            session.begin_sending();

            out.submit_command(npr::Command::SendToLlmProvider {
                payload: SendToLlmProvider {
                    session_id: evt.session_id.clone(),
                    messages,
                },
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::AppState;
    use npr::Command;
    use npr::Event;
    use npr::chat_input::{EnqueueUserMessage, SetChatInputText};
    use npr::provider::{StreamCompleted, StreamCompletedReason};
    use nullslop_component_core::Bus;
    use nullslop_protocol as npr;

    use super::*;

    fn session_id(state: &AppState) -> npr::SessionId {
        state.active_session.clone()
    }

    #[test]
    fn enqueue_when_idle_dispatches_immediately() {
        // Given a bus with MessageQueueHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
        MessageQueueHandler.register(&mut bus);

        let mut state = AppState::new();
        let sid = session_id(&state);

        // When processing EnqueueUserMessage while idle.
        bus.submit_command(Command::EnqueueUserMessage {
            payload: EnqueueUserMessage {
                session_id: sid.clone(),
                text: "hello".to_owned(),
            },
        });
        bus.process_commands(&mut state);

        // Then the entry is in history and is_sending is true.
        assert_eq!(state.session(&sid).history().len(), 1);
        assert_eq!(
            state.session(&sid).history()[0].kind,
            npr::ChatEntryKind::User("hello".to_owned())
        );
        assert!(state.session(&sid).is_sending());

        // And a SendToLlmProvider command was submitted.
        let commands = bus.drain_processed_commands();
        let send = commands
            .iter()
            .find(|c| matches!(c.command, Command::SendToLlmProvider { .. }));
        assert!(send.is_some(), "expected SendToLlmProvider command");
    }

    #[test]
    fn enqueue_when_busy_queues_message() {
        // Given a bus with MessageQueueHandler registered and a busy session.
        let mut bus: Bus<AppState> = Bus::new();
        MessageQueueHandler.register(&mut bus);

        let mut state = AppState::new();
        let sid = session_id(&state);
        state.session_mut(&sid).begin_sending();

        // When processing EnqueueUserMessage while busy.
        bus.submit_command(Command::EnqueueUserMessage {
            payload: EnqueueUserMessage {
                session_id: sid.clone(),
                text: "queued msg".to_owned(),
            },
        });
        bus.process_commands(&mut state);

        // Then no new entry is added to history.
        assert_eq!(state.session(&sid).history().len(), 0);

        // But the message is in the queue.
        assert_eq!(state.session(&sid).queue_len(), 1);
        assert_eq!(state.session(&sid).queue()[0], "queued msg");
    }

    #[test]
    fn stream_completed_dispatches_next_from_queue() {
        // Given a bus with MessageQueueHandler registered, a busy session with a queued message.
        let mut bus: Bus<AppState> = Bus::new();
        MessageQueueHandler.register(&mut bus);

        let mut state = AppState::new();
        let sid = session_id(&state);
        state.session_mut(&sid).begin_sending();
        state
            .session_mut(&sid)
            .enqueue_message("next msg".to_owned());

        // When processing StreamCompleted(Finished).
        bus.submit_event(Event::StreamCompleted {
            payload: StreamCompleted {
                session_id: sid.clone(),
                reason: StreamCompletedReason::Finished,
            },
        });
        bus.process_events(&mut state);
        bus.process_commands(&mut state);

        // Then the queued message was dispatched: history has it, queue is empty, is_sending is true.
        assert_eq!(state.session(&sid).history().len(), 1);
        assert_eq!(
            state.session(&sid).history()[0].kind,
            npr::ChatEntryKind::User("next msg".to_owned())
        );
        assert_eq!(state.session(&sid).queue_len(), 0);
        assert!(state.session(&sid).is_sending());

        // And a SendToLlmProvider command was submitted.
        let commands = bus.drain_processed_commands();
        let send = commands
            .iter()
            .find(|c| matches!(c.command, Command::SendToLlmProvider { .. }));
        assert!(send.is_some());
    }

    #[test]
    fn stream_completed_canceled_does_not_dispatch() {
        // Given a bus with MessageQueueHandler registered, a session that was cancelled.
        let mut bus: Bus<AppState> = Bus::new();
        MessageQueueHandler.register(&mut bus);

        let mut state = AppState::new();
        let sid = session_id(&state);
        state.session_mut(&sid).begin_sending();
        // Cancel already drained the queue in the cancel handler.
        // Simulate: queue is empty, session is still "sending" (cancel_streaming clears it).
        state.session_mut(&sid).cancel_streaming();

        // When processing StreamCompleted(Canceled).
        bus.submit_event(Event::StreamCompleted {
            payload: StreamCompleted {
                session_id: sid.clone(),
                reason: StreamCompletedReason::Canceled,
            },
        });
        bus.process_events(&mut state);
        bus.process_commands(&mut state);

        // Then no dispatch happened: no history, no pending commands.
        assert_eq!(state.session(&sid).history().len(), 0);
        assert!(!bus.has_pending());
    }

    #[test]
    fn cancel_stream_drains_queue_and_restores_input() {
        // Given a bus with MessageQueueHandler registered and queued messages.
        let mut bus: Bus<AppState> = Bus::new();
        MessageQueueHandler.register(&mut bus);

        let mut state = AppState::new();
        let sid = session_id(&state);
        state.session_mut(&sid).begin_streaming();
        state.session_mut(&sid).enqueue_message("first".to_owned());
        state.session_mut(&sid).enqueue_message("second".to_owned());

        // When processing CancelStream.
        bus.submit_command(Command::CancelStream {
            payload: npr::provider::CancelStream {
                session_id: sid.clone(),
            },
        });
        bus.process_commands(&mut state);

        // Then queue is drained and streaming is cancelled.
        assert_eq!(state.session(&sid).queue_len(), 0);
        assert!(!state.session(&sid).is_streaming());

        // And a SetChatInputText command was submitted with concatenated text.
        let commands = bus.drain_processed_commands();
        let set_text = commands
            .iter()
            .find(|c| matches!(c.command, Command::SetChatInputText { .. }));
        assert!(set_text.is_some(), "expected SetChatInputText command");
        match &set_text.unwrap().command {
            Command::SetChatInputText { payload } => {
                assert_eq!(payload.text, "first\nsecond");
            }
            _ => panic!("expected SetChatInputText"),
        }
    }

    #[test]
    fn set_chat_input_text_replaces_input_buffer() {
        // Given a bus with MessageQueueHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
        MessageQueueHandler.register(&mut bus);

        let mut state = AppState::new();
        let sid = session_id(&state);

        // When processing SetChatInputText.
        bus.submit_command(Command::SetChatInputText {
            payload: SetChatInputText {
                session_id: sid,
                text: "restored text".to_owned(),
            },
        });
        bus.process_commands(&mut state);

        // Then the input buffer is updated.
        assert_eq!(state.active_chat_input().text(), "restored text");
    }
}
