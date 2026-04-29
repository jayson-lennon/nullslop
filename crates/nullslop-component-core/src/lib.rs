//! Runtime dispatch infrastructure for the component system.
//!
//! Commands and events flow through a central [`Bus`] to typed handlers.
//! Handlers react to specific message types via the [`CommandHandler`] and
//! [`EventHandler`] traits, and can produce new messages through an [`Out`]
//! buffer. The [`define_handler!`](crate::define_handler) macro reduces
//! boilerplate when declaring handlers.

pub mod bus;
pub mod fake;
pub mod handler;
mod macros;
pub mod out;

pub use bus::Bus;
pub use handler::{CommandHandler, EventHandler};
pub use out::Out;
