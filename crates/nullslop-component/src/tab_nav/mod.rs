//! Tab navigation — handles switching between application tabs.
//!
//! Listens for the [`SwitchTab`] command and updates the active tab in state.

pub(crate) mod handler;

use crate::{AppBus, AppUiRegistry};

/// Registers the tab navigation handler.
pub(crate) fn register(bus: &mut AppBus, _registry: &mut AppUiRegistry) {
    handler::TabNavHandler.register(bus);
}
