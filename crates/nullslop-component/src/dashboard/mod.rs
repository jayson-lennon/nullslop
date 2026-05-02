//! Dashboard component — displays registered actors and their status.
//!
//! Shows a list of all actors known to the application along with their
//! startup lifecycle status ("Starting" or "Started"). The dashboard updates
//! in real-time as actors progress through the startup sequence.

pub(crate) mod element;
pub(crate) mod handler;
pub mod state;

pub use element::DashboardElement;
pub use state::DashboardState;

use crate::{AppBus, AppUiRegistry};

/// Registers the dashboard handler and UI element.
pub(crate) fn register(bus: &mut AppBus, registry: &mut AppUiRegistry) {
    handler::DashboardHandler.register(bus);
    registry.register(Box::new(DashboardElement));
}
