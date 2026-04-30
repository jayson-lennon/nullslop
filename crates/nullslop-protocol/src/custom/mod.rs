//! Extension command and event routing infrastructure.
//!
//! Extensions register their own commands and events through the
//! [`CommandMsg`] and [`EventMsg`] traits. This domain provides the
//! routing infrastructure ([`CustomCommand`], [`EventCustom`]) for
//! extension-defined message types.

mod command;
mod event;

pub use command::{CommandMsg, CustomCommand};
pub use event::{EventCustom, EventMsg};
