//! Commands that mutate the chat input buffer.
//!
//! Insertion, deletion, submission, and clearing of the text
//! the user is composing.

use serde::{Deserialize, Serialize};

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

/// Move the cursor one grapheme to the left.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatBoxMoveCursorLeft;

/// Move the cursor one grapheme to the right.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatBoxMoveCursorRight;

/// Move the cursor to the beginning of the input buffer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatBoxMoveCursorToStart;

/// Move the cursor to the end of the input buffer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatBoxMoveCursorToEnd;

/// Delete the grapheme after the cursor (forward delete).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatBoxDeleteGraphemeForward;

/// Move the cursor one word to the left.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatBoxMoveCursorWordLeft;

/// Move the cursor one word to the right.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatBoxMoveCursorWordRight;
