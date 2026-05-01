//! System domain: application-level commands, events, and built-in actor commands.

mod command;
mod event;

pub use command::{EditInput, Quit, SetMode, ToggleWhichKey};
pub use event::{KeyDown, KeyUp, ModeChanged};
