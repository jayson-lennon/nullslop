//! Application message type for the processing loop.

use nullslop_protocol::{ActorName, Command, Event};

/// An application message for the core processing loop.
#[derive(Debug)]
pub enum AppMsg {
    /// A command to be routed through the bus.
    Command {
        /// The command payload.
        command: Command,
        /// The actor that submitted this command, if any.
        source: Option<ActorName>,
    },
    /// An event from an actor (routed through the bus).
    Event {
        /// The event payload.
        event: Event,
        /// The actor that submitted this event, if any.
        source: Option<ActorName>,
    },
}
