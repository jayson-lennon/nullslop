//! Tab commands.

use serde::{Deserialize, Serialize};

use super::TabDirection;

/// Switch to a different tab.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSwitchTab {
    /// The direction to cycle tabs.
    pub direction: TabDirection,
}
