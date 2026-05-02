//! Provider picker component — filter and select LLM providers.
//!
//! Manages the picker overlay state and handles keyboard input for
//! filtering, navigating, and confirming provider selection.

pub mod entries;
pub mod handler;
pub mod state;

pub use state::ProviderPickerState;

use nullslop_component_core::Bus;
use nullslop_component_ui::UiRegistry;

use crate::AppState;

/// Register the provider picker component with the bus.
///
/// The picker has no UI element — it is rendered as an overlay in
/// `nullslop-tui/src/render.rs`.
pub(crate) fn register(bus: &mut Bus<AppState>, _registry: &mut UiRegistry<AppState>) {
    handler::PickerHandler.register(bus);
}
