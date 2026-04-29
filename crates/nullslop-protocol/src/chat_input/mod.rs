//! Chat input domain: commands and events for the chat input box.

mod command;
mod event;

pub use command::{ChatBoxClear, ChatBoxDeleteGrapheme, ChatBoxInsertChar, ChatBoxSubmitMessage};
pub use event::EventChatMessageSubmitted;
