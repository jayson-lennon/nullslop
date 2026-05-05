//! Handles streaming LLM tokens and tool call updates.
//!
//! Processes [`StreamToken`] commands to update session streaming state,
//! and [`ToolUseStarted`], [`ToolCallStreaming`], [`ToolCallReceived`],
//! and [`PushToolResult`] commands to manage in-progress tool call entries
//! in the chat log.

use crate::AppState;
use npr::CommandAction;
use npr::provider::StreamToken;
use npr::tool::{PushToolResult, ToolCallReceived, ToolCallStreaming, ToolUseStarted};
use nullslop_component_core::{HandlerContext, define_handler};
use nullslop_protocol as npr;
use nullslop_services::Services;

define_handler! {
    pub(crate) struct ProviderHandler;

    commands {
        StreamToken: on_stream_token,
        ToolUseStarted: on_tool_use_started,
        ToolCallStreaming: on_tool_call_streaming,
        ToolCallReceived: on_tool_call_received,
        PushToolResult: on_push_tool_result,
    }

    events {}
}

/// Start streaming on the session if not already active.
///
/// Handles the edge case where tool call events arrive before any text
/// tokens — whichever arrives first begins the streaming session.
fn ensure_streaming(session: &mut crate::ChatSessionState) {
    if !session.is_streaming() {
        if session.is_sending() {
            session.finish_sending();
        }
        session.begin_streaming();
    }
}

impl ProviderHandler {
    /// Appends a streaming LLM token to the session, transitioning to streaming on the first token.
    fn on_stream_token(
        cmd: &StreamToken,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        let session = ctx.state.session_mut(&cmd.session_id);

        if cmd.index == 0 && !session.is_streaming() {
            ensure_streaming(session);
        }

        session.append_stream_token(&cmd.token);
        CommandAction::Continue
    }

    /// Creates a placeholder `ToolCall` entry when a tool use begins in the stream.
    fn on_tool_use_started(
        cmd: &ToolUseStarted,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        let session = ctx.state.session_mut(&cmd.session_id);
        ensure_streaming(session);
        session.begin_tool_call(cmd.index, &cmd.id, &cmd.name);
        CommandAction::Continue
    }

    /// Appends an incremental argument delta to an in-progress tool call entry.
    fn on_tool_call_streaming(
        cmd: &ToolCallStreaming,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        let session = ctx.state.session_mut(&cmd.session_id);
        session.append_tool_call_delta(cmd.index, &cmd.partial_json);
        CommandAction::Continue
    }

    /// Finalizes a tool call entry by overwriting arguments with the complete value.
    fn on_tool_call_received(
        cmd: &ToolCallReceived,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        let session = ctx.state.session_mut(&cmd.session_id);
        session.finalize_tool_call(
            &cmd.tool_call.id,
            &cmd.tool_call.name,
            &cmd.tool_call.arguments,
        );
        CommandAction::Continue
    }

    /// Adds a tool result entry to the session history.
    fn on_push_tool_result(
        cmd: &PushToolResult,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        let session = ctx.state.session_mut(&cmd.session_id);
        let entry = npr::ChatEntry::tool_result(
            &cmd.result.tool_call_id,
            &cmd.result.name,
            &cmd.result.content,
            cmd.result.success,
        );
        session.push_entry(entry);
        CommandAction::Continue
    }
}

#[cfg(test)]
mod tests {
    use crate::AppState;
    use npr::Command;
    use npr::provider::StreamToken;
    use npr::tool::{
        PushToolResult, ToolCall, ToolCallReceived, ToolCallStreaming, ToolUseStarted,
    };
    use npr::{ChatEntryKind, ToolResult};
    use nullslop_component_core::Bus;
    use nullslop_protocol as npr;
    use nullslop_services::Services;

    use super::*;
    use crate::test_utils;

    fn session_id(state: &AppState) -> npr::SessionId {
        state.active_session.clone()
    }

    #[test]
    fn stream_token_appends_to_assistant_entry() {
        // Given a bus with ProviderHandler registered.
        let mut bus: Bus<AppState, Services> = Bus::new();
        ProviderHandler.register(&mut bus);

        let services = test_utils::test_services();
        let mut state = AppState::default();
        let sid = session_id(&state);

        // When processing StreamToken(index=0, token="Hello").
        bus.submit_command(Command::StreamToken {
            payload: StreamToken {
                session_id: sid.clone(),
                index: 0,
                token: "Hello".to_owned(),
            },
        });
        bus.process_commands(&mut state, &services);

        // Then the session has an Assistant entry with "Hello".
        assert!(state.active_session().is_streaming());
        assert_eq!(
            state.active_session().history()[0].kind,
            ChatEntryKind::Assistant("Hello".to_owned())
        );

        // When processing another StreamToken(index=1, token=" world").
        bus.submit_command(Command::StreamToken {
            payload: StreamToken {
                session_id: sid.clone(),
                index: 1,
                token: " world".to_owned(),
            },
        });
        bus.process_commands(&mut state, &services);

        // Then the text is "Hello world".
        assert_eq!(
            state.active_session().history()[0].kind,
            ChatEntryKind::Assistant("Hello world".to_owned())
        );
    }

    #[test]
    fn stream_token_clears_sending_on_first_token() {
        // Given a bus with ProviderHandler registered and a session that is sending.
        let mut bus: Bus<AppState, Services> = Bus::new();
        ProviderHandler.register(&mut bus);

        let services = test_utils::test_services();
        let mut state = AppState::default();
        let sid = session_id(&state);
        state.session_mut(&sid).begin_sending();
        assert!(state.session(&sid).is_sending());

        // When processing the first StreamToken.
        bus.submit_command(Command::StreamToken {
            payload: StreamToken {
                session_id: sid.clone(),
                index: 0,
                token: "Hi".to_owned(),
            },
        });
        bus.process_commands(&mut state, &services);

        // Then is_sending is cleared and is_streaming is set.
        assert!(!state.session(&sid).is_sending());
        assert!(state.session(&sid).is_streaming());
    }

    // --- Tool call handler tests ---

    #[test]
    fn tool_use_started_creates_tool_call_entry() {
        // Given a bus with ProviderHandler registered.
        let mut bus: Bus<AppState, Services> = Bus::new();
        ProviderHandler.register(&mut bus);

        let services = test_utils::test_services();
        let mut state = AppState::default();
        let sid = session_id(&state);

        // When processing a ToolUseStarted command.
        bus.submit_command(Command::ToolUseStarted {
            payload: ToolUseStarted {
                session_id: sid.clone(),
                index: 0,
                id: "call_1".to_owned(),
                name: "echo".to_owned(),
            },
        });
        bus.process_commands(&mut state, &services);

        // Then the session is streaming and has a ToolCall entry with empty arguments.
        assert!(state.session(&sid).is_streaming());
        assert_eq!(state.session(&sid).history().len(), 2); // assistant + tool call
        assert_eq!(
            state.session(&sid).history()[1].kind,
            ChatEntryKind::ToolCall {
                id: "call_1".to_owned(),
                name: "echo".to_owned(),
                arguments: String::new(),
            }
        );
    }

    #[test]
    fn tool_use_started_starts_streaming_if_not_active() {
        // Given a bus with ProviderHandler registered and a session that is sending (not streaming).
        let mut bus: Bus<AppState, Services> = Bus::new();
        ProviderHandler.register(&mut bus);

        let services = test_utils::test_services();
        let mut state = AppState::default();
        let sid = session_id(&state);
        state.session_mut(&sid).begin_sending();
        assert!(!state.session(&sid).is_streaming());

        // When processing a ToolUseStarted command.
        bus.submit_command(Command::ToolUseStarted {
            payload: ToolUseStarted {
                session_id: sid.clone(),
                index: 0,
                id: "call_1".to_owned(),
                name: "echo".to_owned(),
            },
        });
        bus.process_commands(&mut state, &services);

        // Then the session is now streaming (transitioned from sending).
        assert!(state.session(&sid).is_streaming());
        assert!(!state.session(&sid).is_sending());
    }

    #[test]
    fn tool_call_streaming_appends_arguments() {
        // Given a bus with ProviderHandler registered and a session with a tool call started.
        let mut bus: Bus<AppState, Services> = Bus::new();
        ProviderHandler.register(&mut bus);

        let services = test_utils::test_services();
        let mut state = AppState::default();
        let sid = session_id(&state);

        bus.submit_command(Command::ToolUseStarted {
            payload: ToolUseStarted {
                session_id: sid.clone(),
                index: 0,
                id: "call_1".to_owned(),
                name: "echo".to_owned(),
            },
        });
        bus.process_commands(&mut state, &services);

        // When processing ToolCallStreaming deltas.
        bus.submit_command(Command::ToolCallStreaming {
            payload: ToolCallStreaming {
                session_id: sid.clone(),
                index: 0,
                partial_json: r#"{"input":"#.to_owned(),
            },
        });
        bus.process_commands(&mut state, &services);

        bus.submit_command(Command::ToolCallStreaming {
            payload: ToolCallStreaming {
                session_id: sid.clone(),
                index: 0,
                partial_json: r#""hello"}"#.to_owned(),
            },
        });
        bus.process_commands(&mut state, &services);

        // Then the tool call entry has accumulated the deltas.
        assert_eq!(
            state.session(&sid).history()[1].kind,
            ChatEntryKind::ToolCall {
                id: "call_1".to_owned(),
                name: "echo".to_owned(),
                arguments: r#"{"input":"hello"}"#.to_owned(),
            }
        );
    }

    #[test]
    fn tool_call_received_finalizes_entry() {
        // Given a bus with ProviderHandler registered and a session with streamed tool call.
        let mut bus: Bus<AppState, Services> = Bus::new();
        ProviderHandler.register(&mut bus);

        let services = test_utils::test_services();
        let mut state = AppState::default();
        let sid = session_id(&state);

        bus.submit_command(Command::ToolUseStarted {
            payload: ToolUseStarted {
                session_id: sid.clone(),
                index: 0,
                id: "call_1".to_owned(),
                name: "echo".to_owned(),
            },
        });
        bus.process_commands(&mut state, &services);

        bus.submit_command(Command::ToolCallStreaming {
            payload: ToolCallStreaming {
                session_id: sid.clone(),
                index: 0,
                partial_json: r#"{"input":"#.to_owned(),
            },
        });
        bus.process_commands(&mut state, &services);

        // When processing ToolCallReceived with the final complete tool call.
        bus.submit_command(Command::ToolCallReceived {
            payload: ToolCallReceived {
                session_id: sid.clone(),
                tool_call: ToolCall {
                    id: "call_1".to_owned(),
                    name: "echo".to_owned(),
                    arguments: r#"{"input":"world"}"#.to_owned(),
                },
            },
        });
        bus.process_commands(&mut state, &services);

        // Then the tool call entry has the final complete arguments.
        assert_eq!(
            state.session(&sid).history()[1].kind,
            ChatEntryKind::ToolCall {
                id: "call_1".to_owned(),
                name: "echo".to_owned(),
                arguments: r#"{"input":"world"}"#.to_owned(),
            }
        );
    }

    #[test]
    fn push_tool_result_adds_entry_to_history() {
        // Given a bus with ProviderHandler registered.
        let mut bus: Bus<AppState, Services> = Bus::new();
        ProviderHandler.register(&mut bus);

        let services = test_utils::test_services();
        let mut state = AppState::default();
        let sid = session_id(&state);

        // When processing a PushToolResult command.
        bus.submit_command(Command::PushToolResult {
            payload: PushToolResult {
                session_id: sid.clone(),
                result: ToolResult {
                    tool_call_id: "call_1".to_owned(),
                    name: "echo".to_owned(),
                    content: "hello".to_owned(),
                    success: true,
                },
            },
        });
        bus.process_commands(&mut state, &services);

        // Then the session has a ToolResult entry.
        assert_eq!(state.session(&sid).history().len(), 1);
        assert_eq!(
            state.session(&sid).history()[0].kind,
            ChatEntryKind::ToolResult {
                id: "call_1".to_owned(),
                name: "echo".to_owned(),
                content: "hello".to_owned(),
                success: true,
            }
        );
    }

    #[test]
    fn tool_calls_and_text_interleaved() {
        // Given a bus with ProviderHandler registered.
        let mut bus: Bus<AppState, Services> = Bus::new();
        ProviderHandler.register(&mut bus);

        let services = test_utils::test_services();
        let mut state = AppState::default();
        let sid = session_id(&state);

        // When text tokens arrive first, then a tool call.
        bus.submit_command(Command::StreamToken {
            payload: StreamToken {
                session_id: sid.clone(),
                index: 0,
                token: "Let me check".to_owned(),
            },
        });
        bus.process_commands(&mut state, &services);

        bus.submit_command(Command::ToolUseStarted {
            payload: ToolUseStarted {
                session_id: sid.clone(),
                index: 0,
                id: "call_1".to_owned(),
                name: "get_time".to_owned(),
            },
        });
        bus.process_commands(&mut state, &services);

        bus.submit_command(Command::ToolCallStreaming {
            payload: ToolCallStreaming {
                session_id: sid.clone(),
                index: 0,
                partial_json: "{}".to_owned(),
            },
        });
        bus.process_commands(&mut state, &services);

        // Then history has assistant text followed by tool call.
        assert_eq!(state.session(&sid).history().len(), 2);
        assert_eq!(
            state.session(&sid).history()[0].kind,
            ChatEntryKind::Assistant("Let me check".to_owned())
        );
        assert_eq!(
            state.session(&sid).history()[1].kind,
            ChatEntryKind::ToolCall {
                id: "call_1".to_owned(),
                name: "get_time".to_owned(),
                arguments: "{}".to_owned(),
            }
        );
    }

    #[test]
    fn push_tool_result_with_failed_execution() {
        // Given a bus with ProviderHandler registered.
        let mut bus: Bus<AppState, Services> = Bus::new();
        ProviderHandler.register(&mut bus);

        let services = test_utils::test_services();
        let mut state = AppState::default();
        let sid = session_id(&state);

        // When processing a PushToolResult with success=false.
        bus.submit_command(Command::PushToolResult {
            payload: PushToolResult {
                session_id: sid.clone(),
                result: ToolResult {
                    tool_call_id: "call_1".to_owned(),
                    name: "file_read".to_owned(),
                    content: "Permission denied".to_owned(),
                    success: false,
                },
            },
        });
        bus.process_commands(&mut state, &services);

        // Then the entry records the failure.
        assert_eq!(
            state.session(&sid).history()[0].kind,
            ChatEntryKind::ToolResult {
                id: "call_1".to_owned(),
                name: "file_read".to_owned(),
                content: "Permission denied".to_owned(),
                success: false,
            }
        );
    }
}
