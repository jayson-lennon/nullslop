//! Extension lifecycle events.

use serde::{Deserialize, Serialize};

use crate::custom::EventMsg;

/// An extension is starting up.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionStarting {
    /// The extension's name.
    pub name: String,
}

/// An extension has finished starting up.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionStarted {
    /// The extension's name.
    pub name: String,
}

/// An extension has completed shutdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionShutdownCompleted {
    /// The extension's name.
    pub name: String,
}

impl EventMsg for ExtensionStarting {
    const TYPE_NAME: &'static str = "EventExtensionStarting";
}

impl EventMsg for ExtensionStarted {
    const TYPE_NAME: &'static str = "EventExtensionStarted";
}

impl EventMsg for ExtensionShutdownCompleted {
    const TYPE_NAME: &'static str = "EventExtensionShutdownCompleted";
}
