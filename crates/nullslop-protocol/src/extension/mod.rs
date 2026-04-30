//! Extension lifecycle domain: commands and events for extension startup, shutdown coordination.

mod command;
mod event;

pub use command::ProceedWithShutdown;
pub use event::{ExtensionShutdownCompleted, ExtensionStarted, ExtensionStarting};
