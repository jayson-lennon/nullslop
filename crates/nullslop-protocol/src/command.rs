//! Command types for the component command pipeline.
//!
//! The [`Command`] enum is the unified type the host uses to receive and
//! dispatch instructions from both internal handlers and extensions.
//!
//! Individual command structs live in domain modules ([`chat_input`], [`system`],
//! [`custom`], [`shutdown`]). This module re-exports them for convenience.

use serde::{Deserialize, Serialize};

// Re-export command structs and trait from domain modules.
pub use crate::chat_input::{
    ChatBoxClear, ChatBoxDeleteGrapheme, ChatBoxInsertChar, ChatBoxSubmitMessage,
};
pub use crate::custom::{CommandMsg, CustomCommand, EchoCommand};
pub use crate::shutdown::ProceedWithShutdown;
pub use crate::system::{
    AppEditInput, AppQuit, AppSetMode, AppToggleWhichKey, ProviderCancelStream, ProviderSendMessage,
};

/// Every command the host can receive.
///
/// Extensions and internal handlers produce these; the host dispatches
/// them to the appropriate domain handler.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(tag = "type")]
pub enum Command {
    /// Insert a character into the chat input buffer.
    #[serde(rename = "chat_box_insert_char")]
    ChatBoxInsertChar {
        /// Details of the character to insert.
        #[serde(flatten)]
        payload: ChatBoxInsertChar,
    },
    /// Delete the last grapheme from the chat input buffer.
    #[serde(rename = "chat_box_delete_grapheme")]
    ChatBoxDeleteGrapheme,
    /// Submit the chat input buffer as a message.
    #[serde(rename = "chat_box_submit_message")]
    ChatBoxSubmitMessage {
        /// The message being submitted.
        #[serde(flatten)]
        payload: ChatBoxSubmitMessage,
    },
    /// Clear the chat input buffer.
    #[serde(rename = "chat_box_clear")]
    ChatBoxClear,
    /// Set the application interaction mode.
    #[serde(rename = "app_set_mode")]
    AppSetMode {
        /// The target mode.
        #[serde(flatten)]
        payload: AppSetMode,
    },
    /// Quit the application.
    #[serde(rename = "app_quit")]
    AppQuit,
    /// Open an external editor for the input buffer.
    #[serde(rename = "app_edit_input")]
    AppEditInput,
    /// Toggle the which-key popup.
    #[serde(rename = "app_toggle_which_key")]
    AppToggleWhichKey,
    /// Send a message to the AI provider.
    #[serde(rename = "provider_send_message")]
    ProviderSendMessage {
        /// The message to send.
        #[serde(flatten)]
        payload: ProviderSendMessage,
    },
    /// Cancel the active provider stream.
    #[serde(rename = "provider_cancel_stream")]
    ProviderCancelStream,
    /// A custom command from an extension.
    #[serde(rename = "custom_command")]
    CustomCommand {
        /// The extension-defined command name and arguments.
        #[serde(flatten)]
        payload: CustomCommand,
    },
    /// Proceed with shutdown after extension coordination.
    #[serde(rename = "proceed_with_shutdown")]
    ProceedWithShutdown {
        /// Which extensions finished or timed out.
        #[serde(flatten)]
        payload: ProceedWithShutdown,
    },
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::ChatBoxInsertChar { payload } => write!(f, "insert '{}'", payload.ch),
            Command::ChatBoxDeleteGrapheme => write!(f, "delete"),
            Command::ChatBoxSubmitMessage { .. } => write!(f, "submit chat"),
            Command::ChatBoxClear => write!(f, "clear"),
            Command::AppSetMode { payload } => write!(f, "set mode {:?}", payload.mode),
            Command::AppQuit => write!(f, "quit"),
            Command::AppEditInput => write!(f, "edit in $EDITOR"),
            Command::AppToggleWhichKey => write!(f, "toggle which-key"),
            Command::ProviderSendMessage { .. } => write!(f, "send message"),
            Command::ProviderCancelStream => write!(f, "cancel stream"),
            Command::CustomCommand { payload } => write!(f, "{}", payload.name),
            Command::ProceedWithShutdown { payload } => {
                write!(
                    f,
                    "proceed with shutdown ({} completed, {} timed out)",
                    payload.completed.len(),
                    payload.timed_out.len()
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Mode;

    #[test]
    fn command_insert_char_serialization() {
        // Given a ChatBoxInsertChar command.
        let cmd = Command::ChatBoxInsertChar {
            payload: ChatBoxInsertChar { ch: 'a' },
        };

        // When serialized.
        let json = serde_json::to_string(&cmd).expect("serialize");

        // Then it contains the type tag and the character.
        assert!(json.contains(r#""type":"chat_box_insert_char""#));
        assert!(json.contains(r#""ch":"a""#));
    }

    #[test]
    fn command_app_quit_serialization() {
        // Given an AppQuit command.
        let cmd = Command::AppQuit;

        // When serialized.
        let json = serde_json::to_string(&cmd).expect("serialize");

        // Then it is {"type":"app_quit"}.
        assert_eq!(json, r#"{"type":"app_quit"}"#);
    }

    #[test]
    fn command_custom_serialization() {
        // Given a CustomCommand.
        let cmd = Command::CustomCommand {
            payload: CustomCommand {
                name: "echo".to_string(),
                args: serde_json::json!({"text": "hi"}),
            },
        };

        // When serialized and deserialized.
        let json = serde_json::to_string(&cmd).expect("serialize");
        let back: Command = serde_json::from_str(&json).expect("deserialize");

        // Then name is preserved.
        match back {
            Command::CustomCommand { payload } => assert_eq!(payload.name, "echo"),
            other => panic!("expected CustomCommand, got {other:?}"),
        }
    }

    #[rstest::rstest]
    #[case::insert_char(Command::ChatBoxInsertChar { payload: ChatBoxInsertChar { ch: 'x' } })]
    #[case::delete_grapheme(Command::ChatBoxDeleteGrapheme)]
    #[case::submit_message(Command::ChatBoxSubmitMessage { payload: ChatBoxSubmitMessage { text: "hello".into() } })]
    #[case::clear(Command::ChatBoxClear)]
    #[case::set_mode(Command::AppSetMode { payload: AppSetMode { mode: Mode::Input } })]
    #[case::quit(Command::AppQuit)]
    #[case::edit_input(Command::AppEditInput)]
    #[case::toggle_which_key(Command::AppToggleWhichKey)]
    #[case::send_message(Command::ProviderSendMessage { payload: ProviderSendMessage { text: "hi".into() } })]
    #[case::cancel_stream(Command::ProviderCancelStream)]
    #[case::custom(Command::CustomCommand { payload: CustomCommand { name: "echo".into(), args: serde_json::json!({}) } })]
    #[case::proceed_with_shutdown(Command::ProceedWithShutdown { payload: ProceedWithShutdown { completed: vec!["ext-a".into()], timed_out: vec!["ext-b".into()] } })]
    fn command_roundtrip_all_variants(#[case] cmd: Command) {
        // Given a command variant.
        let json = serde_json::to_string(&cmd).expect("serialize");

        // When deserialized.
        let back: Command = serde_json::from_str(&json).expect("deserialize");

        // Then it matches the original when re-serialized.
        let back_json = serde_json::to_string(&back).expect("re-serialize");
        assert_eq!(json, back_json);
    }
}
