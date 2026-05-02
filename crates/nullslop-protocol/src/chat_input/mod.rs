//! Chat input domain: commands and events for the text input box.
//!
//! Users type into the input box to compose messages; these types
//! model the resulting edits and submissions.

mod command;
mod event;

pub use command::{
    Clear, DeleteGrapheme, DeleteGraphemeForward, EnqueueUserMessage, InsertChar, MoveCursorDown,
    MoveCursorLeft, MoveCursorRight, MoveCursorToEnd, MoveCursorToStart, MoveCursorUp,
    MoveCursorWordLeft, MoveCursorWordRight, PushChatEntry, SetChatInputText, SubmitMessage,
};
pub use event::ChatEntrySubmitted;
