//! Tab domain: tab management types, active tab state, and tab navigation commands.

mod active_tab;
mod command;

use serde::{Deserialize, Serialize};

pub use active_tab::ActiveTab;
pub use command::SwitchTab;

/// Direction for tab cycling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TabDirection {
    /// Move to the next tab (wrapping).
    Next,
    /// Move to the previous tab (wrapping).
    Prev,
}
