//! Actor lifecycle domain: commands and events for actor startup, shutdown coordination.

mod command;
mod event;

pub use command::ProceedWithShutdown;
pub use event::{ActorShutdownCompleted, ActorStarted, ActorStarting};
