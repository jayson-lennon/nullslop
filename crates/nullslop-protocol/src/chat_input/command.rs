//! Chat input box commands.

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
