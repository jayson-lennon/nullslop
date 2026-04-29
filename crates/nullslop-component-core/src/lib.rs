//! nullslop-component-core: handler traits, bus dispatch, and buffered output.
//!
//! This crate provides the runtime dispatch infrastructure for the component system:
//! - [`CommandHandler`] and [`EventHandler`] traits for typed handling
//! - [`Out`] for buffered command/event submission
//! - [`Bus`] for [`TypeId`]-keyed dispatch with processing loops
//! - [`define_handler!`](crate::define_handler) macro for declarative handler definitions
//! - [`AppState`], [`ShutdownTracker`], and [`ChatInputBoxState`] for shared state
//!
//! It depends only on `nullslop-protocol` — no dependency on `nullslop-core`.

pub mod app_state;
pub mod bus;
pub mod chat_input_state;
pub mod fake;
pub mod handler;
mod macros;
pub mod out;

pub use app_state::{AppState, ShutdownTracker};
pub use bus::Bus;
pub use chat_input_state::ChatInputBoxState;
pub use handler::{CommandHandler, EventHandler};
pub use out::Out;
