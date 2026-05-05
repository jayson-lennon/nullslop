//! System commands.

use serde::{Deserialize, Serialize};

use crate::CommandMsg;
use crate::Mode;

/// Set the application interaction mode.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("system")]
pub struct SetMode {
    /// The mode to switch to.
    pub mode: Mode,
}

/// Quit the application.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("system")]
pub struct Quit;

/// Open an external editor for the input buffer.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("system")]
pub struct EditInput;

/// Toggle the which-key popup.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("system")]
pub struct ToggleWhichKey;

/// Scroll the chat log up (toward older messages).
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("system")]
pub struct ScrollUp;

/// Scroll the chat log down (toward newer messages).
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("system")]
pub struct ScrollDown;

/// Scroll the chat log up by a small amount (mouse wheel).
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("system")]
pub struct MouseScrollUp;

/// Scroll the chat log down by a small amount (mouse wheel).
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("system")]
pub struct MouseScrollDown;

/// Scroll the chat log up by one line.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("system")]
pub struct ScrollLineUp;

/// Scroll the chat log down by one line.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("system")]
pub struct ScrollLineDown;

/// Scroll the chat log to the very top.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("system")]
pub struct ScrollToTop;

/// Scroll the chat log to the very bottom.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("system")]
pub struct ScrollToBottom;

/// Move the dashboard selection down one entry.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("system")]
pub struct DashboardSelectDown;

/// Move the dashboard selection up one entry.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("system")]
pub struct DashboardSelectUp;

/// Move the dashboard selection to the first entry.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("system")]
pub struct DashboardSelectFirst;

/// Move the dashboard selection to the last entry.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("system")]
pub struct DashboardSelectLast;
