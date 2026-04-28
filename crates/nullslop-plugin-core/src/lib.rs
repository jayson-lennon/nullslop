//! nullslop-plugin-core: handler traits, bus dispatch, and buffered output.
//!
//! This crate provides the runtime dispatch infrastructure for the plugin system:
//! - [`CommandHandler`] and [`EventHandler`] traits for typed handling
//! - [`Out`] for buffered command/event submission
//! - [`Bus`] for [`TypeId`]-keyed dispatch with processing loops
//! - [`define_plugin!`](crate::define_plugin) macro for declarative plugin definitions
//!
//! It depends only on `nullslop-protocol` — no dependency on `nullslop-core`.

pub mod bus;
pub mod fake;
pub mod handler;
mod macros;
pub mod out;

pub use bus::Bus;
pub use handler::{CommandHandler, EventHandler};
pub use out::Out;
