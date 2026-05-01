//! Provider domain: commands, events, and LLM message types.

mod command;
mod convert;
mod event;
mod message;

pub use command::{CancelStream, SendMessage, SendToLlmProvider, StreamToken};
pub use convert::entries_to_messages;
pub use event::{StreamCompleted, StreamCompletedReason};
pub use message::{LlmMessage, LlmRole};
