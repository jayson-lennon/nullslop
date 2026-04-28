//! Domain-level message type for the application processing loop.
//!
//! [`AppMsg`] represents the messages that [`AppCore`](crate::AppCore) processes.
//! Unlike the TUI's `Msg` enum, it contains no crossterm types or render ticks.

use crate::{Command, RegisteredExtension};

/// A domain-level message for the application core.
#[derive(Debug)]
pub enum AppMsg {
    /// A command to be routed through the bus.
    Command(Command),
    /// Extensions have completed discovery and registration.
    ExtensionsReady(Vec<RegisteredExtension>),
}
