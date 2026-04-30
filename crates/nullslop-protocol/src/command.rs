//! Command types for the component command pipeline.
//!
//! The [`Command`] enum is the unified type the host uses to receive and
//! dispatch instructions from both internal handlers and extensions.
//!
//! Individual command structs live in domain modules ([`chat_input`], [`system`],
//! [`custom`], [`shutdown`]). This module re-exports them for convenience.
//!
//! # When adding a new command
//!
//! Every new command struct **must** be added as a variant on the [`Command`] enum
//! below. Creating the struct alone is not enough — the bus dispatches based on
//! enum variants, so a missing variant means the command is invisible to the system.

use serde::{Deserialize, Serialize};

// Re-export command structs and trait from domain modules.
pub use crate::chat_input::{
    ChatBoxClear, ChatBoxDeleteGrapheme, ChatBoxDeleteGraphemeForward, ChatBoxInsertChar,
    ChatBoxMoveCursorLeft, ChatBoxMoveCursorRight, ChatBoxMoveCursorToEnd,
    ChatBoxMoveCursorToStart, ChatBoxMoveCursorWordLeft, ChatBoxMoveCursorWordRight,
    ChatBoxSubmitMessage,
};
pub use crate::custom::{CommandMsg, CustomCommand, EchoCommand};
pub use crate::shutdown::ProceedWithShutdown;
pub use crate::system::{
    AppEditInput, AppQuit, AppSetMode, AppSwitchTab, AppToggleWhichKey, ProviderCancelStream,
    ProviderSendMessage, TabDirection,
};

/// Every command the host can receive.
///
/// Extensions and internal handlers produce these; the host dispatches
/// them to the appropriate domain handler.
///
/// **When adding a new command struct**, you must add a corresponding variant to
/// this enum. A command struct defined in a domain module without an enum variant
/// here will not be dispatched by the bus.
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
    /// Move the cursor one grapheme to the left.
    #[serde(rename = "chat_box_move_cursor_left")]
    ChatBoxMoveCursorLeft,
    /// Move the cursor one grapheme to the right.
    #[serde(rename = "chat_box_move_cursor_right")]
    ChatBoxMoveCursorRight,
    /// Move the cursor to the beginning of the input buffer.
    #[serde(rename = "chat_box_move_cursor_to_start")]
    ChatBoxMoveCursorToStart,
    /// Move the cursor to the end of the input buffer.
    #[serde(rename = "chat_box_move_cursor_to_end")]
    ChatBoxMoveCursorToEnd,
    /// Delete the grapheme after the cursor (forward delete).
    #[serde(rename = "chat_box_delete_grapheme_forward")]
    ChatBoxDeleteGraphemeForward,
    /// Move the cursor one word to the left.
    #[serde(rename = "chat_box_move_cursor_word_left")]
    ChatBoxMoveCursorWordLeft,
    /// Move the cursor one word to the right.
    #[serde(rename = "chat_box_move_cursor_word_right")]
    ChatBoxMoveCursorWordRight,
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
    /// Switch to a different tab.
    #[serde(rename = "app_switch_tab")]
    AppSwitchTab {
        /// The tab to switch to.
        #[serde(flatten)]
        payload: AppSwitchTab,
    },
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
            Command::ChatBoxMoveCursorLeft => write!(f, "cursor left"),
            Command::ChatBoxMoveCursorRight => write!(f, "cursor right"),
            Command::ChatBoxMoveCursorToStart => write!(f, "cursor home"),
            Command::ChatBoxMoveCursorToEnd => write!(f, "cursor end"),
            Command::ChatBoxDeleteGraphemeForward => write!(f, "forward delete"),
            Command::ChatBoxMoveCursorWordLeft => write!(f, "cursor word left"),
            Command::ChatBoxMoveCursorWordRight => write!(f, "cursor word right"),
            Command::AppSetMode { payload } => write!(f, "set mode {:?}", payload.mode),
            Command::AppQuit => write!(f, "quit"),
            Command::AppEditInput => write!(f, "edit in $EDITOR"),
            Command::AppToggleWhichKey => write!(f, "toggle which-key"),
            Command::AppSwitchTab { payload } => write!(f, "switch tab {:?}", payload.direction),
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
    #[case::switch_tab(Command::AppSwitchTab { payload: AppSwitchTab { direction: crate::TabDirection::Next } })]
    #[case::send_message(Command::ProviderSendMessage { payload: ProviderSendMessage { text: "hi".into() } })]
    #[case::cancel_stream(Command::ProviderCancelStream)]
    #[case::custom(Command::CustomCommand { payload: CustomCommand { name: "echo".into(), args: serde_json::json!({}) } })]
    #[case::proceed_with_shutdown(Command::ProceedWithShutdown { payload: ProceedWithShutdown { completed: vec!["ext-a".into()], timed_out: vec!["ext-b".into()] } })]
    #[case::move_cursor_left(Command::ChatBoxMoveCursorLeft)]
    #[case::move_cursor_right(Command::ChatBoxMoveCursorRight)]
    #[case::move_cursor_to_start(Command::ChatBoxMoveCursorToStart)]
    #[case::move_cursor_to_end(Command::ChatBoxMoveCursorToEnd)]
    #[case::delete_forward(Command::ChatBoxDeleteGraphemeForward)]
    #[case::move_cursor_word_left(Command::ChatBoxMoveCursorWordLeft)]
    #[case::move_cursor_word_right(Command::ChatBoxMoveCursorWordRight)]
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
