//! nullslop-core: application runtime for the nullslop agent harness.
//!
//! The processing loop ([`AppCore`]) receives messages, routes commands and events
//! through a component bus, and forwards them to the actor host. Shared state
//! ([`State`]) is accessible from any thread via read/write guards. The actor
//! host ([`ActorHost`]) manages lifecycle and message routing for actors.
//!
//! Protocol types (commands, events, keys, chat entries) are in `nullslop-protocol`.
//! Import them directly from there.

pub mod actor_sink;
pub mod app_core;
pub mod app_msg;
pub mod state;

// Re-export primary types owned by this crate
pub use actor_sink::ActorMessageSink;
pub use app_core::{AppCore, SHUTDOWN_TIMEOUT, TickResult};
pub use app_msg::AppMsg;
pub use state::{State, StateReadGuard, StateWriteGuard};

// Re-export actor host types for convenience
pub use nullslop_actor_host::{ActorHost, ActorHostService};

// Re-export protocol types for convenience (types that don't have a module shadowing them)
pub use nullslop_protocol::{CommandAction, Mode};
