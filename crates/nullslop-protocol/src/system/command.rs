//! System commands.

use serde::{Deserialize, Serialize};

use crate::Mode;

/// Set the application interaction mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSetMode {
    /// The mode to switch to.
    pub mode: Mode,
}

/// Quit the application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppQuit;

/// Open an external editor for the input buffer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEditInput;

/// Toggle the which-key popup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppToggleWhichKey;

/// Send a message to the AI provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSendMessage {
    /// The message text.
    pub text: String,
}

/// Cancel the active provider stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCancelStream;
