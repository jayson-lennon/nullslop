//! Custom command/event domain: trait definitions, custom command types, and `EchoCommand`.

mod command;
mod event;

pub use command::{CommandMsg, CustomCommand, EchoCommand};
pub use event::{EventCustom, EventMsg};
