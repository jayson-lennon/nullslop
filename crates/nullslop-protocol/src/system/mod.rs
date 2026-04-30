//! System domain: application-level commands, events, and built-in extension commands.

mod command;
mod event;

pub use command::{AppEditInput, AppQuit, AppSetMode, AppToggleWhichKey, EchoCommand};
pub use event::{EventApplicationReady, EventApplicationShuttingDown, EventKeyDown, EventKeyUp, EventModeChanged};
