//! System commands.

use serde::{Deserialize, Serialize};

use crate::Mode;
use crate::custom::CommandMsg;

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

/// The echo command.
pub struct EchoCommand;

impl CommandMsg for EchoCommand {
    const NAME: &'static str = "echo";
}
