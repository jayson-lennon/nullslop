//! Handles streaming LLM tokens.
//!
//! Processes [`StreamToken`] commands to update session streaming state.
//! On the first token, transitions from sending to streaming.

use crate::AppState;
use npr::CommandAction;
use npr::provider::StreamToken;
use nullslop_component_core::{Out, define_handler};
use nullslop_protocol as npr;

define_handler! {
    pub(crate) struct ProviderHandler;

    commands {
        StreamToken: on_stream_token,
    }

    events {}
}

impl ProviderHandler {
    fn on_stream_token(cmd: &StreamToken, state: &mut AppState, _out: &mut Out) -> CommandAction {
        let session = state.session_mut(&cmd.session_id);

        if cmd.index == 0 && !session.is_streaming() {
            // First token arrived — transition from sending to streaming.
            if session.is_sending() {
                session.finish_sending();
            }
            session.begin_streaming();
        }

        session.append_stream_token(&cmd.token);
        CommandAction::Continue
    }
}

#[cfg(test)]
mod tests {
    use crate::AppState;
    use npr::Command;
    use npr::provider::StreamToken;
    use nullslop_component_core::Bus;
    use nullslop_protocol as npr;

    use super::*;

    fn session_id(state: &AppState) -> npr::SessionId {
        state.active_session.clone()
    }

    #[test]
    fn stream_token_appends_to_assistant_entry() {
        // Given a bus with ProviderHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
        ProviderHandler.register(&mut bus);

        let mut state = AppState::new();
        let sid = session_id(&state);

        // When processing StreamToken(index=0, token="Hello").
        bus.submit_command(Command::StreamToken {
            payload: StreamToken {
                session_id: sid.clone(),
                index: 0,
                token: "Hello".to_string(),
            },
        });
        bus.process_commands(&mut state);

        // Then the session has an Assistant entry with "Hello".
        assert!(state.active_session().is_streaming());
        assert_eq!(
            state.active_session().history()[0].kind,
            npr::ChatEntryKind::Assistant("Hello".to_string())
        );

        // When processing another StreamToken(index=1, token=" world").
        bus.submit_command(Command::StreamToken {
            payload: StreamToken {
                session_id: sid.clone(),
                index: 1,
                token: " world".to_string(),
            },
        });
        bus.process_commands(&mut state);

        // Then the text is "Hello world".
        assert_eq!(
            state.active_session().history()[0].kind,
            npr::ChatEntryKind::Assistant("Hello world".to_string())
        );
    }

    #[test]
    fn stream_token_clears_sending_on_first_token() {
        // Given a bus with ProviderHandler registered and a session that is sending.
        let mut bus: Bus<AppState> = Bus::new();
        ProviderHandler.register(&mut bus);

        let mut state = AppState::new();
        let sid = session_id(&state);
        state.session_mut(&sid).begin_sending();
        assert!(state.session(&sid).is_sending());

        // When processing the first StreamToken.
        bus.submit_command(Command::StreamToken {
            payload: StreamToken {
                session_id: sid.clone(),
                index: 0,
                token: "Hi".to_string(),
            },
        });
        bus.process_commands(&mut state);

        // Then is_sending is cleared and is_streaming is set.
        assert!(!state.session(&sid).is_sending());
        assert!(state.session(&sid).is_streaming());
    }
}
