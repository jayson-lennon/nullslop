//! Custom commands and the `CommandMsg` trait.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Marker trait for extension commands that provides compile-time-checked names.
///
/// Each implementation provides a [`NAME`](CommandMsg::NAME) constant
/// used for command routing and `CustomCommand` construction.
pub trait CommandMsg: Send + Sync + 'static {
    /// The command name used for routing.
    const NAME: &'static str;
}

/// A custom command from an extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomCommand {
    /// The command name.
    pub name: String,
    /// The command arguments.
    pub args: Value,
}

/// The echo command.
pub struct EchoCommand;

impl CommandMsg for EchoCommand {
    const NAME: &'static str = "echo";
}
