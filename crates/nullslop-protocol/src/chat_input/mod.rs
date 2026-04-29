//! Chat input domain: commands, events, and state for the chat input box.

mod command;
mod event;
mod state;

pub use command::{ChatBoxClear, ChatBoxDeleteGrapheme, ChatBoxInsertChar, ChatBoxSubmitMessage};
pub use event::EventChatMessageSubmitted;
pub use state::ChatInputBoxState;
