//! Tab commands.

use serde::{Deserialize, Serialize};

use super::TabDirection;
use crate::CommandMsg;

/// Switch to a different tab.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("tab")]
pub struct SwitchTab {
    /// The direction to cycle tabs.
    pub direction: TabDirection,
}
