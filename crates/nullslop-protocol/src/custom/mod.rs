//! Actor command and event routing infrastructure.
//!
//! The [`CommandMsg`] and [`EventMsg`] traits provide compile-time routing
//! strings used by the derive macros in domain modules.

mod command;
mod derive_tests;
mod event;

pub use command::{CommandMsg, CommandName};
pub use event::{EventMsg, EventTypeName};
