//! Graceful shutdown coordination for actors.
//!
//! Ensures the application doesn't exit until every running actor has had a
//! chance to clean up. When shutdown is triggered, this component waits for each
//! actor to report completion before allowing the application to proceed with
//! exiting.

pub mod handler;
pub mod state;

use crate::AppBus;
use crate::AppUiRegistry;

pub(crate) use handler::ShutdownTrackerHandler;
pub use state::ShutdownTrackerState;

/// Register the shutdown tracker handler.
pub(crate) fn register(bus: &mut AppBus, _registry: &mut AppUiRegistry) {
    ShutdownTrackerHandler.register(bus);
}
