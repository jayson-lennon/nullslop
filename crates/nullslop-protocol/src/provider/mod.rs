//! Provider domain: commands, events, and LLM message types.

mod command;
mod convert;
mod event;
mod message;

pub use command::{
    CancelStream, ProviderSwitch, RefreshModels, SendMessage, SendToLlmProvider, StreamToken,
};
pub use convert::entries_to_messages;
pub use event::{ModelsRefreshed, ProviderSwitched, StreamCompleted, StreamCompletedReason};
pub use message::LlmMessage;
