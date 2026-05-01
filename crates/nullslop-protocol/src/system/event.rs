//! System events.

use serde::{Deserialize, Serialize};

use crate::EventMsg;
use crate::Mode;
use crate::key::KeyEvent;

/// A key was pressed down.
#[derive(Debug, Clone, Serialize, Deserialize, EventMsg)]
#[event_msg("system")]
pub struct KeyDown {
    /// The key event.
    pub key: KeyEvent,
}

/// A key was released.
#[derive(Debug, Clone, Serialize, Deserialize, EventMsg)]
#[event_msg("system")]
pub struct KeyUp {
    /// The key event.
    pub key: KeyEvent,
}

/// The application mode changed.
#[derive(Debug, Clone, Serialize, Deserialize, EventMsg)]
#[event_msg("system")]
pub struct ModeChanged {
    /// The previous mode.
    pub from: Mode,
    /// The new mode.
    pub to: Mode,
}
