//! Shutdown domain: commands and events for extension lifecycle and shutdown coordination.

mod command;
mod event;

pub use command::ProceedWithShutdown;
pub use event::{
    EventApplicationShuttingDown, ExtensionShutdownCompleted, ExtensionStarted, ExtensionStarting,
};
