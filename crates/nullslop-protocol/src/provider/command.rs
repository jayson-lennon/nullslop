//! Provider commands.

use serde::{Deserialize, Serialize};

/// Send a message to the AI provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSendMessage {
    /// The message text.
    pub text: String,
}

/// Cancel the active provider stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCancelStream;
