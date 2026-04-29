//! Events produced when the chat input box emits a completed message.

use serde::{Deserialize, Serialize};

use crate::ChatEntry;
use crate::custom::EventMsg;

/// A chat message was submitted by the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventChatMessageSubmitted {
    /// The chat entry that was submitted.
    pub entry: ChatEntry,
}

impl EventMsg for EventChatMessageSubmitted {
    const TYPE_NAME: &'static str = "EventChatMessageSubmitted";
}
