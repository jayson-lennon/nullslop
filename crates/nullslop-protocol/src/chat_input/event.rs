//! Events produced when a chat entry is added to the conversation.

use serde::{Deserialize, Serialize};

use crate::ChatEntry;
use crate::EventMsg;

/// A chat entry was added to the conversation history.
#[derive(Debug, Clone, Serialize, Deserialize, EventMsg)]
#[event_msg("chat_input")]
pub struct ChatEntrySubmitted {
    /// The chat entry that was added.
    pub entry: ChatEntry,
}
