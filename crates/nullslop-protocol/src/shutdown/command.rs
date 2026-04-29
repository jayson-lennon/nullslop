//! Shutdown commands.

use serde::{Deserialize, Serialize};

/// Proceed with shutdown after extension coordination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProceedWithShutdown {
    /// Extensions that completed shutdown successfully.
    pub completed: Vec<String>,
    /// Extensions that did not respond before timeout.
    pub timed_out: Vec<String>,
}
