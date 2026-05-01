//! Commands that mutate the chat input buffer.
//!
//! Insertion, deletion, submission, and clearing of the text
//! the user is composing.

use serde::{Deserialize, Serialize};

use crate::ChatEntry;
use crate::CommandMsg;

/// Insert a character into the chat input buffer.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("chat_input")]
pub struct InsertChar {
    /// The character to insert.
    pub ch: char,
}

/// Delete the last grapheme from the chat input buffer.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("chat_input")]
pub struct DeleteGrapheme;

/// Submit the chat input buffer as a message.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("chat_input")]
pub struct SubmitMessage {
    /// The message text.
    pub text: String,
}

/// Clear the chat input buffer.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("chat_input")]
pub struct Clear;

/// Move the cursor one grapheme to the left.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("chat_input")]
pub struct MoveCursorLeft;

/// Move the cursor one grapheme to the right.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("chat_input")]
pub struct MoveCursorRight;

/// Move the cursor to the beginning of the input buffer.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("chat_input")]
pub struct MoveCursorToStart;

/// Move the cursor to the end of the input buffer.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("chat_input")]
pub struct MoveCursorToEnd;

/// Delete the grapheme after the cursor (forward delete).
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("chat_input")]
pub struct DeleteGraphemeForward;

/// Move the cursor one word to the left.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("chat_input")]
pub struct MoveCursorWordLeft;

/// Move the cursor one word to the right.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("chat_input")]
pub struct MoveCursorWordRight;

/// Push a chat entry into the conversation history.
///
/// Any component or actor can send this to add an entry to the chat log.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("chat_input")]
pub struct PushChatEntry {
    /// The chat entry to add.
    pub entry: ChatEntry,
}
