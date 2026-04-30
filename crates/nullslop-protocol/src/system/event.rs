//! System events.

use serde::{Deserialize, Serialize};

use crate::Mode;
use crate::custom::EventMsg;
use crate::key::KeyEvent;

/// A key was pressed down.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventKeyDown {
    /// The key event.
    pub key: KeyEvent,
}

/// A key was released.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventKeyUp {
    /// The key event.
    pub key: KeyEvent,
}

/// The application mode changed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventModeChanged {
    /// The previous mode.
    pub from: Mode,
    /// The new mode.
    pub to: Mode,
}

/// The application has finished starting up.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventApplicationReady;

/// The application is shutting down.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventApplicationShuttingDown;

impl EventMsg for EventApplicationReady {
    const TYPE_NAME: &'static str = "EventApplicationReady";
}

impl EventMsg for EventApplicationShuttingDown {
    const TYPE_NAME: &'static str = "EventApplicationShuttingDown";
}
