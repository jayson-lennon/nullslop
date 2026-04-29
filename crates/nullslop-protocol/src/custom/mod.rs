//! Extension-defined commands and events.
//!
//! Extensions register their own commands and events through the
//! [`CommandMsg`] and [`EventMsg`] traits. This domain provides the
//! routing infrastructure and built-in types like [`EchoCommand`].

mod command;
mod event;

pub use command::{CommandMsg, CustomCommand, EchoCommand};
pub use event::{EventCustom, EventMsg};
