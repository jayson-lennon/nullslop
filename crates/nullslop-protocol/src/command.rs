//! Command types for the plugin command pipeline.
//!
//! Each command is a separate struct with a component prefix (`ChatBox*`,
//! `App*`, `Provider*`). The [`Command`] wrapper enum provides a single
//! type for serialization and the wire protocol.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Mode;

/// Insert a character into the chat input buffer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatBoxInsertChar {
    /// The character to insert.
    pub ch: char,
}

/// Delete the last grapheme from the chat input buffer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatBoxDeleteGrapheme;

/// Submit the chat input buffer as a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatBoxSubmitMessage {
    /// The message text.
    pub text: String,
}

/// Clear the chat input buffer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatBoxClear;

/// Set the application interaction mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSetMode {
    /// The mode to switch to.
    pub mode: Mode,
}

/// Quit the application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppQuit;

/// Open an external editor for the input buffer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEditInput;

/// Toggle the which-key popup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppToggleWhichKey;

/// Send a message to the AI provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSendMessage {
    /// The message text.
    pub text: String,
}

/// Cancel the active provider stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCancelStream;

/// A custom command from an extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomCommand {
    /// The command name.
    pub name: String,
    /// The command arguments.
    pub args: Value,
}

/// Wrapper enum for all commands.
///
/// Used for serialization and the wire protocol between host and extensions.
/// Each variant wraps its corresponding command struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(tag = "type")]
pub enum Command {
    /// Insert a character into the chat input buffer.
    #[serde(rename = "chat_box_insert_char")]
    ChatBoxInsertChar {
        /// The command payload.
        #[serde(flatten)]
        payload: ChatBoxInsertChar,
    },
    /// Delete the last grapheme from the chat input buffer.
    #[serde(rename = "chat_box_delete_grapheme")]
    ChatBoxDeleteGrapheme,
    /// Submit the chat input buffer as a message.
    #[serde(rename = "chat_box_submit_message")]
    ChatBoxSubmitMessage {
        /// The command payload.
        #[serde(flatten)]
        payload: ChatBoxSubmitMessage,
    },
    /// Clear the chat input buffer.
    #[serde(rename = "chat_box_clear")]
    ChatBoxClear,
    /// Set the application interaction mode.
    #[serde(rename = "app_set_mode")]
    AppSetMode {
        /// The command payload.
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
        /// The command payload.
        #[serde(flatten)]
        payload: ProviderSendMessage,
    },
    /// Cancel the active provider stream.
    #[serde(rename = "provider_cancel_stream")]
    ProviderCancelStream,
    /// A custom command from an extension.
    #[serde(rename = "custom_command")]
    CustomCommand {
        /// The command payload.
        #[serde(flatten)]
        payload: CustomCommand,
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
