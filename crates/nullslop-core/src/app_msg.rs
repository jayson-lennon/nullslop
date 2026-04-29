//! Application message type for the processing loop.
//!
//! [`AppMsg`] represents the messages that [`AppCore`](crate::AppCore) processes.
//! Unlike the TUI's `Msg` enum, it contains no crossterm types or render ticks.

use crate::{Command, Event, RegisteredExtension};

/// An application message for the core processing loop.
#[derive(Debug)]
pub enum AppMsg {
    /// A command to be routed through the bus.
    Command(Command),
    /// An event from an extension (routed through the bus).
    Event(Event),
    /// Extensions have completed discovery and registration.
    ExtensionsReady(Vec<RegisteredExtension>),
}
