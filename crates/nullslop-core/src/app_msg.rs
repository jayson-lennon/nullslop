//! Application message type for the processing loop.

use nullslop_protocol::{Command, Event, ExtensionName};

use crate::RegisteredExtension;

/// An application message for the core processing loop.
#[derive(Debug)]
pub enum AppMsg {
    /// A command to be routed through the bus.
    Command {
        /// The command payload.
        command: Command,
        /// The extension that submitted this command, if any.
        source: Option<ExtensionName>,
    },
    /// An event from an extension (routed through the bus).
    Event {
        /// The event payload.
        event: Event,
        /// The extension that submitted this event, if any.
        source: Option<ExtensionName>,
    },
    /// Extensions have completed discovery and registration.
    ExtensionsReady(Vec<RegisteredExtension>),
}
