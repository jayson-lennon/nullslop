//! Provider commands.

use serde::{Deserialize, Serialize};

use crate::CommandMsg;

/// Send a message to the AI provider.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("provider")]
pub struct SendMessage {
    /// The message text.
    pub text: String,
}

/// Cancel the active provider stream.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("provider")]
pub struct CancelStream;
