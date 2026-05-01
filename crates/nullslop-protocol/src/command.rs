//! Command types for the component command pipeline.
//!
//! The [`Command`] enum is the unified type the host uses to receive and
//! dispatch instructions from both internal handlers and actors.
//!
//! Individual command structs live in domain modules ([`chat_input`], [`system`],
//! [`custom`], [`actor`], [`provider`], [`tab`]). Consumers import structs
//! directly from those modules — this facade only re-exports infrastructure types.
//!
//! # When adding a new command
//!
//! Every new command struct **must** be added as a variant on the [`Command`] enum
//! below. Creating the struct alone is not enough — the bus dispatches based on
//! enum variants, so a missing variant means the command is invisible to the system.

use serde::{Deserialize, Serialize};

// Re-export infrastructure types only. Domain structs are imported from their modules.
pub use crate::custom::CommandMsg;

// Internal imports for enum definition and Display impl.
use crate::actor::ProceedWithShutdown;
use crate::chat_input::{InsertChar, PushChatEntry, SubmitMessage};
use crate::provider::SendMessage;
use crate::system::SetMode;
use crate::tab::SwitchTab;

/// Every command the host can receive.
///
/// Actors and internal handlers produce these; the host dispatches
/// them to the appropriate domain handler.
///
/// **When adding a new command struct**, you must add a corresponding variant to
/// this enum. A command struct defined in a domain module without an enum variant
/// here will not be dispatched by the bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Command {
    /// Insert a character into the chat input buffer.
    #[serde(rename = "insert_char")]
    InsertChar {
        /// Details of the character to insert.
        #[serde(flatten)]
        payload: InsertChar,
    },
    /// Delete the last grapheme from the chat input buffer.
    #[serde(rename = "delete_grapheme")]
    DeleteGrapheme,
    /// Submit the chat input buffer as a message.
    #[serde(rename = "submit_message")]
    SubmitMessage {
        /// The message being submitted.
        #[serde(flatten)]
        payload: SubmitMessage,
    },
    /// Clear the chat input buffer.
    #[serde(rename = "clear")]
    Clear,
    /// Move the cursor one grapheme to the left.
    #[serde(rename = "move_cursor_left")]
    MoveCursorLeft,
    /// Move the cursor one grapheme to the right.
    #[serde(rename = "move_cursor_right")]
    MoveCursorRight,
    /// Move the cursor to the beginning of the input buffer.
    #[serde(rename = "move_cursor_to_start")]
    MoveCursorToStart,
    /// Move the cursor to the end of the input buffer.
    #[serde(rename = "move_cursor_to_end")]
    MoveCursorToEnd,
    /// Delete the grapheme after the cursor (forward delete).
    #[serde(rename = "delete_grapheme_forward")]
    DeleteGraphemeForward,
    /// Move the cursor one word to the left.
    #[serde(rename = "move_cursor_word_left")]
    MoveCursorWordLeft,
    /// Move the cursor one word to the right.
    #[serde(rename = "move_cursor_word_right")]
    MoveCursorWordRight,
    /// Set the application interaction mode.
    #[serde(rename = "set_mode")]
    SetMode {
        /// The target mode.
        #[serde(flatten)]
        payload: SetMode,
    },
    /// Quit the application.
    #[serde(rename = "quit")]
    Quit,
    /// Open an external editor for the input buffer.
    #[serde(rename = "edit_input")]
    EditInput,
    /// Toggle the which-key popup.
    #[serde(rename = "toggle_which_key")]
    ToggleWhichKey,
    /// Switch to a different tab.
    #[serde(rename = "switch_tab")]
    SwitchTab {
        /// The tab to switch to.
        #[serde(flatten)]
        payload: SwitchTab,
    },
    /// Send a message to the AI provider.
    #[serde(rename = "send_message")]
    SendMessage {
        /// The message to send.
        #[serde(flatten)]
        payload: SendMessage,
    },
    /// Cancel the active provider stream.
    #[serde(rename = "cancel_stream")]
    CancelStream,
    /// Push a chat entry into the conversation history.
    #[serde(rename = "push_chat_entry")]
    PushChatEntry {
        /// The chat entry to add.
        #[serde(flatten)]
        payload: PushChatEntry,
    },
    /// Proceed with shutdown after actor coordination.
    #[serde(rename = "proceed_with_shutdown")]
    ProceedWithShutdown {
        /// Which actors finished or timed out.
        #[serde(flatten)]
        payload: ProceedWithShutdown,
    },
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::InsertChar { payload } => write!(f, "insert '{}'", payload.ch),
            Command::DeleteGrapheme => write!(f, "delete"),
            Command::SubmitMessage { .. } => write!(f, "submit chat"),
            Command::Clear => write!(f, "clear"),
            Command::MoveCursorLeft => write!(f, "cursor left"),
            Command::MoveCursorRight => write!(f, "cursor right"),
            Command::MoveCursorToStart => write!(f, "cursor home"),
            Command::MoveCursorToEnd => write!(f, "cursor end"),
            Command::DeleteGraphemeForward => write!(f, "forward delete"),
            Command::MoveCursorWordLeft => write!(f, "cursor word left"),
            Command::MoveCursorWordRight => write!(f, "cursor word right"),
            Command::SetMode { payload } => write!(f, "set mode {:?}", payload.mode),
            Command::Quit => write!(f, "quit"),
            Command::EditInput => write!(f, "edit in $EDITOR"),
            Command::ToggleWhichKey => write!(f, "toggle which-key"),
            Command::SwitchTab { payload } => write!(f, "switch tab {:?}", payload.direction),
            Command::SendMessage { .. } => write!(f, "send message"),
            Command::CancelStream => write!(f, "cancel stream"),
            Command::PushChatEntry { .. } => write!(f, "push chat entry"),
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
        // Given an InsertChar command.
        let cmd = Command::InsertChar {
            payload: InsertChar { ch: 'a' },
        };

        // When serialized.
        let json = serde_json::to_string(&cmd).expect("serialize");

        // Then it contains the type tag and the character.
        assert!(json.contains(r#""type":"insert_char""#));
        assert!(json.contains(r#""ch":"a""#));
    }

    #[test]
    fn command_app_quit_serialization() {
        // Given a Quit command.
        let cmd = Command::Quit;

        // When serialized.
        let json = serde_json::to_string(&cmd).expect("serialize");

        // Then it is {"type":"quit"}.
        assert_eq!(json, r#"{"type":"quit"}"#);
    }

    #[rstest::rstest]
    #[case::insert_char(Command::InsertChar { payload: InsertChar { ch: 'x' } })]
    #[case::delete_grapheme(Command::DeleteGrapheme)]
    #[case::submit_message(Command::SubmitMessage { payload: SubmitMessage { text: "hello".into() } })]
    #[case::clear(Command::Clear)]
    #[case::set_mode(Command::SetMode { payload: SetMode { mode: Mode::Input } })]
    #[case::quit(Command::Quit)]
    #[case::edit_input(Command::EditInput)]
    #[case::toggle_which_key(Command::ToggleWhichKey)]
    #[case::switch_tab(Command::SwitchTab { payload: SwitchTab { direction: crate::TabDirection::Next } })]
    #[case::send_message(Command::SendMessage { payload: SendMessage { text: "hi".into() } })]
    #[case::cancel_stream(Command::CancelStream)]
    #[case::push_chat_entry(Command::PushChatEntry { payload: PushChatEntry { entry: crate::ChatEntry::user("hi") } })]
    #[case::proceed_with_shutdown(Command::ProceedWithShutdown { payload: ProceedWithShutdown { completed: vec!["ext-a".into()], timed_out: vec!["ext-b".into()] } })]
    #[case::move_cursor_left(Command::MoveCursorLeft)]
    #[case::move_cursor_right(Command::MoveCursorRight)]
    #[case::move_cursor_to_start(Command::MoveCursorToStart)]
    #[case::move_cursor_to_end(Command::MoveCursorToEnd)]
    #[case::delete_forward(Command::DeleteGraphemeForward)]
    #[case::move_cursor_word_left(Command::MoveCursorWordLeft)]
    #[case::move_cursor_word_right(Command::MoveCursorWordRight)]
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
