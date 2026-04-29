//! Application message type for the processing loop.

use nullslop_protocol::{Command, Event};

use crate::RegisteredExtension;

/// An application message for the core processing loop.
#[derive(Debug)]
pub enum AppMsg {
    /// A command to be routed through the bus.
    Command {
        /// The command payload.
        command: Command,
        /// The extension name that submitted this command, if any.
        source: Option<String>,
    },
    /// An event from an extension (routed through the bus).
    Event {
        /// The event payload.
        event: Event,
        /// The extension name that submitted this event, if any.
        source: Option<String>,
    },
    /// Extensions have completed discovery and registration.
    ExtensionsReady(Vec<RegisteredExtension>),
}
