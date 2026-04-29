//! Application shutdown component.
//!
//! Responsible for gracefully ending the application when the user requests it.
//! Once triggered, no further command processing occurs.

pub mod handler;

use crate::AppBus;
use crate::AppUiRegistry;

pub(crate) use handler::AppQuitHandler;

/// Register the app quit handler.
pub(crate) fn register(bus: &mut AppBus, _registry: &mut AppUiRegistry) {
    AppQuitHandler.register(bus);
}
