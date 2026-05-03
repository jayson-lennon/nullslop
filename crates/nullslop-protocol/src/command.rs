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
use crate::chat_input::{
    Clear, DeleteGrapheme, DeleteGraphemeForward, EnqueueUserMessage, InsertChar, Interrupt,
    MoveCursorDown, MoveCursorToEnd, MoveCursorToStart, MoveCursorUp, MoveCursorWordLeft,
    MoveCursorWordRight, PushChatEntry, SetChatInputText, SubmitMessage,
};
use crate::chat_input::{MoveCursorLeft, MoveCursorRight};
use crate::provider::{
    CancelStream, ProviderSwitch, RefreshModels, SendMessage, SendToLlmProvider, StreamToken,
};
use crate::provider_picker::{
    PickerBackspace, PickerConfirm, PickerInsertChar, PickerMoveCursorLeft, PickerMoveCursorRight,
    PickerMoveDown, PickerMoveUp,
};
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
    /// Context-sensitive interrupt: clear input if non-empty, otherwise quit.
    #[serde(rename = "interrupt")]
    Interrupt,
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
    /// Move the cursor up one visual line.
    #[serde(rename = "move_cursor_up")]
    MoveCursorUp,
    /// Move the cursor down one visual line.
    #[serde(rename = "move_cursor_down")]
    MoveCursorDown,
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
    CancelStream {
        /// The cancel stream command.
        #[serde(flatten)]
        payload: CancelStream,
    },
    /// Send conversation context to the LLM provider.
    #[serde(rename = "send_to_llm_provider")]
    SendToLlmProvider {
        /// The full conversation history as LLM messages.
        #[serde(flatten)]
        payload: SendToLlmProvider,
    },
    /// A single token from a streaming LLM response.
    #[serde(rename = "stream_token")]
    StreamToken {
        /// The stream token.
        #[serde(flatten)]
        payload: StreamToken,
    },
    /// Push a chat entry into the conversation history.
    #[serde(rename = "push_chat_entry")]
    PushChatEntry {
        /// The chat entry to add.
        #[serde(flatten)]
        payload: PushChatEntry,
    },
    /// Enqueue a user message for queued processing.
    #[serde(rename = "enqueue_user_message")]
    EnqueueUserMessage {
        /// The message to enqueue.
        #[serde(flatten)]
        payload: EnqueueUserMessage,
    },
    /// Set the chat input buffer text directly.
    #[serde(rename = "set_chat_input_text")]
    SetChatInputText {
        /// The new input text.
        #[serde(flatten)]
        payload: SetChatInputText,
    },
    /// Proceed with shutdown after actor coordination.
    #[serde(rename = "proceed_with_shutdown")]
    ProceedWithShutdown {
        /// Which actors finished or timed out.
        #[serde(flatten)]
        payload: ProceedWithShutdown,
    },
    /// Switch the active LLM provider.
    #[serde(rename = "provider_switch")]
    ProviderSwitch {
        /// The provider switch details.
        #[serde(flatten)]
        payload: ProviderSwitch,
    },
    /// Scroll the chat log up (toward older messages).
    #[serde(rename = "scroll_up")]
    ScrollUp,
    /// Scroll the chat log down (toward newer messages).
    #[serde(rename = "scroll_down")]
    ScrollDown,
    /// Refresh the model list from all providers.
    #[serde(rename = "refresh_models")]
    RefreshModels,
    /// Insert a character into the picker filter.
    #[serde(rename = "picker_insert_char")]
    PickerInsertChar {
        /// The character to insert.
        #[serde(flatten)]
        payload: PickerInsertChar,
    },
    /// Delete the last character from the picker filter.
    #[serde(rename = "picker_backspace")]
    PickerBackspace,
    /// Confirm the current picker selection.
    #[serde(rename = "picker_confirm")]
    PickerConfirm,
    /// Move the picker selection up.
    #[serde(rename = "picker_move_up")]
    PickerMoveUp,
    /// Move the picker selection down.
    #[serde(rename = "picker_move_down")]
    PickerMoveDown,
    /// Move the picker filter cursor left.
    #[serde(rename = "picker_move_cursor_left")]
    PickerMoveCursorLeft,
    /// Move the picker filter cursor right.
    #[serde(rename = "picker_move_cursor_right")]
    PickerMoveCursorRight,
}

impl Command {
    /// Returns the routing name for this command, if it has one.
    ///
    /// Analogous to [`Event::type_name()`]. Returns `None` for commands
    /// that are not routed to actors (e.g., internal UI commands).
    #[must_use]
    pub fn command_name(&self) -> Option<&'static str> {
        match self {
            Self::InsertChar { .. } => Some(InsertChar::NAME),
            Self::DeleteGrapheme => Some(DeleteGrapheme::NAME),
            Self::SubmitMessage { .. } => Some(SubmitMessage::NAME),
            Self::Clear => Some(Clear::NAME),
            Self::Interrupt => Some(Interrupt::NAME),
            Self::MoveCursorLeft => Some(MoveCursorLeft::NAME),
            Self::MoveCursorRight => Some(MoveCursorRight::NAME),
            Self::MoveCursorToStart => Some(MoveCursorToStart::NAME),
            Self::MoveCursorToEnd => Some(MoveCursorToEnd::NAME),
            Self::DeleteGraphemeForward => Some(DeleteGraphemeForward::NAME),
            Self::MoveCursorWordLeft => Some(MoveCursorWordLeft::NAME),
            Self::MoveCursorWordRight => Some(MoveCursorWordRight::NAME),
            Self::MoveCursorUp => Some(MoveCursorUp::NAME),
            Self::MoveCursorDown => Some(MoveCursorDown::NAME),
            Self::SetMode { .. } => Some(SetMode::NAME),
            Self::Quit
            | Self::EditInput
            | Self::ToggleWhichKey
            | Self::ScrollUp
            | Self::ScrollDown => None,
            Self::SwitchTab { .. } => Some(SwitchTab::NAME),
            Self::SendMessage { .. } => Some(SendMessage::NAME),
            Self::CancelStream { .. } => Some(CancelStream::NAME),
            Self::SendToLlmProvider { .. } => Some(SendToLlmProvider::NAME),
            Self::StreamToken { .. } => Some(StreamToken::NAME),
            Self::PushChatEntry { .. } => Some(PushChatEntry::NAME),
            Self::EnqueueUserMessage { .. } => Some(EnqueueUserMessage::NAME),
            Self::SetChatInputText { .. } => Some(SetChatInputText::NAME),
            Self::ProceedWithShutdown { .. } => Some(ProceedWithShutdown::NAME),
            Self::ProviderSwitch { .. } => Some(ProviderSwitch::NAME),
            Self::RefreshModels => Some(RefreshModels::NAME),
            Self::PickerInsertChar { .. } => Some(PickerInsertChar::NAME),
            Self::PickerBackspace => Some(PickerBackspace::NAME),
            Self::PickerConfirm => Some(PickerConfirm::NAME),
            Self::PickerMoveUp => Some(PickerMoveUp::NAME),
            Self::PickerMoveDown => Some(PickerMoveDown::NAME),
            Self::PickerMoveCursorLeft => Some(PickerMoveCursorLeft::NAME),
            Self::PickerMoveCursorRight => Some(PickerMoveCursorRight::NAME),
        }
    }
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::InsertChar { payload } => write!(f, "insert '{}'", payload.ch),
            Command::DeleteGrapheme => write!(f, "delete"),
            Command::SubmitMessage { .. } => write!(f, "submit chat"),
            Command::Clear => write!(f, "clear"),
            Command::Interrupt => write!(f, "interrupt"),
            Command::MoveCursorLeft => write!(f, "cursor left"),
            Command::MoveCursorRight => write!(f, "cursor right"),
            Command::MoveCursorToStart => write!(f, "cursor home"),
            Command::MoveCursorToEnd => write!(f, "cursor end"),
            Command::DeleteGraphemeForward => write!(f, "forward delete"),
            Command::MoveCursorWordLeft => write!(f, "cursor word left"),
            Command::MoveCursorWordRight => write!(f, "cursor word right"),
            Command::MoveCursorUp => write!(f, "cursor up"),
            Command::MoveCursorDown => write!(f, "cursor down"),
            Command::SetMode { payload } => write!(f, "set mode {}", payload.mode),
            Command::Quit => write!(f, "quit"),
            Command::EditInput => write!(f, "edit in $EDITOR"),
            Command::ToggleWhichKey => write!(f, "toggle which-key"),
            Command::SwitchTab { payload } => write!(f, "switch tab {}", payload.direction),
            Command::SendMessage { .. } => write!(f, "send message"),
            Command::CancelStream { .. } => write!(f, "cancel stream"),
            Command::SendToLlmProvider { .. } => write!(f, "send to LLM provider"),
            Command::StreamToken { payload } => {
                write!(
                    f,
                    "stream token '{}' (idx {})",
                    payload.token, payload.index
                )
            }
            Command::PushChatEntry { .. } => write!(f, "push chat entry"),
            Command::EnqueueUserMessage { .. } => write!(f, "enqueue user message"),
            Command::SetChatInputText { .. } => write!(f, "set chat input text"),
            Command::ProceedWithShutdown { payload } => {
                write!(
                    f,
                    "proceed with shutdown ({} completed, {} timed out)",
                    payload.completed.len(),
                    payload.timed_out.len()
                )
            }
            Command::ProviderSwitch { payload } => {
                write!(f, "provider switch to '{}'", payload.provider_id)
            }
            Command::ScrollUp => write!(f, "scroll up"),
            Command::ScrollDown => write!(f, "scroll down"),
            Command::RefreshModels => write!(f, "refresh models"),
            Command::PickerInsertChar { payload } => write!(f, "picker insert '{}'", payload.ch),
            Command::PickerBackspace => write!(f, "picker backspace"),
            Command::PickerConfirm => write!(f, "picker confirm"),
            Command::PickerMoveUp => write!(f, "picker move up"),
            Command::PickerMoveDown => write!(f, "picker move down"),
            Command::PickerMoveCursorLeft => write!(f, "picker cursor left"),
            Command::PickerMoveCursorRight => write!(f, "picker cursor right"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Mode;
    use crate::SessionId;

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
    #[case::submit_message(Command::SubmitMessage { payload: SubmitMessage { session_id: SessionId::new(), text: "hello".into() } })]
    #[case::clear(Command::Clear)]
    #[case::interrupt(Command::Interrupt)]
    #[case::set_mode(Command::SetMode { payload: SetMode { mode: Mode::Input } })]
    #[case::quit(Command::Quit)]
    #[case::edit_input(Command::EditInput)]
    #[case::toggle_which_key(Command::ToggleWhichKey)]
    #[case::switch_tab(Command::SwitchTab { payload: SwitchTab { direction: crate::TabDirection::Next } })]
    #[case::send_message(Command::SendMessage { payload: SendMessage { session_id: SessionId::new(), text: "hi".into() } })]
    #[case::cancel_stream(Command::CancelStream { payload: CancelStream { session_id: SessionId::new() } })]
    #[case::send_to_llm_provider(Command::SendToLlmProvider { payload: SendToLlmProvider { session_id: SessionId::new(), messages: vec![], provider_id: None } })]
    #[case::stream_token(Command::StreamToken { payload: StreamToken { session_id: SessionId::new(), index: 0, token: "hello".into() } })]
    #[case::push_chat_entry(Command::PushChatEntry { payload: PushChatEntry { session_id: SessionId::new(), entry: crate::ChatEntry::user("hi") } })]
    #[case::proceed_with_shutdown(Command::ProceedWithShutdown { payload: ProceedWithShutdown { completed: vec!["ext-a".into()], timed_out: vec!["ext-b".into()] } })]
    #[case::move_cursor_left(Command::MoveCursorLeft)]
    #[case::move_cursor_right(Command::MoveCursorRight)]
    #[case::move_cursor_to_start(Command::MoveCursorToStart)]
    #[case::move_cursor_to_end(Command::MoveCursorToEnd)]
    #[case::delete_forward(Command::DeleteGraphemeForward)]
    #[case::move_cursor_word_left(Command::MoveCursorWordLeft)]
    #[case::move_cursor_word_right(Command::MoveCursorWordRight)]
    #[case::enqueue_user_message(Command::EnqueueUserMessage { payload: EnqueueUserMessage { session_id: SessionId::new(), text: "hello".into() } })]
    #[case::set_chat_input_text(Command::SetChatInputText { payload: SetChatInputText { session_id: SessionId::new(), text: "restored".into() } })]
    #[case::provider_switch(Command::ProviderSwitch { payload: ProviderSwitch { provider_id: "ollama".into() } })]
    #[case::scroll_up(Command::ScrollUp)]
    #[case::scroll_down(Command::ScrollDown)]
    #[case::move_cursor_up(Command::MoveCursorUp)]
    #[case::move_cursor_down(Command::MoveCursorDown)]
    #[case::picker_insert_char(Command::PickerInsertChar { payload: PickerInsertChar { ch: 'x' } })]
    #[case::picker_backspace(Command::PickerBackspace)]
    #[case::picker_confirm(Command::PickerConfirm)]
    #[case::picker_move_up(Command::PickerMoveUp)]
    #[case::picker_move_down(Command::PickerMoveDown)]
    #[case::picker_move_cursor_left(Command::PickerMoveCursorLeft)]
    #[case::picker_move_cursor_right(Command::PickerMoveCursorRight)]
    #[case::refresh_models(Command::RefreshModels)]
    fn command_roundtrip_all_variants(#[case] cmd: Command) {
        // Given a command variant.
        let json = serde_json::to_string(&cmd).expect("serialize");

        // When deserialized.
        let back: Command = serde_json::from_str(&json).expect("deserialize");

        // Then it matches the original when re-serialized.
        let back_json = serde_json::to_string(&back).expect("re-serialize");
        assert_eq!(json, back_json);
    }

    #[test]
    fn command_name_returns_name_for_routable_commands() {
        // Given routable command variants.
        // When calling command_name().
        // Then they return their routing name.
        assert_eq!(
            Command::PushChatEntry {
                payload: PushChatEntry {
                    session_id: SessionId::new(),
                    entry: crate::ChatEntry::user("test"),
                },
            }
            .command_name(),
            Some(PushChatEntry::NAME)
        );
        assert_eq!(
            Command::CancelStream {
                payload: CancelStream {
                    session_id: SessionId::new(),
                },
            }
            .command_name(),
            Some(CancelStream::NAME)
        );
    }

    #[test]
    fn command_name_returns_none_for_internal_commands() {
        // Given internal UI commands.
        // When calling command_name().
        // Then they return None (not routed to actors).
        assert_eq!(Command::Quit.command_name(), None);
        assert_eq!(Command::EditInput.command_name(), None);
        assert_eq!(Command::ToggleWhichKey.command_name(), None);
    }
}
