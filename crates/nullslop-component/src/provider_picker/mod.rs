//! Provider picker component — filter and select LLM providers.
//!
//! Manages the picker overlay state and handles keyboard input for
//! filtering, navigating, and confirming provider selection.
//!
//! The picker uses [`SelectionState`]`<`[`PickerEntry`]`>` from the
//! `nullslop-selection-widget` crate for all state management and rendering.
//! A [`PickerKind`] dispatch on [`AppState`](crate::AppState) determines which
//! picker is active when commands arrive.

pub mod entries;
pub mod handler;

pub use entries::PickerEntry;
pub use handler::load_provider_picker_items;

use crate::{AppBus, AppUiRegistry};

/// Register the provider picker component with the bus.
///
/// The picker has no UI element — it is rendered as an overlay in
/// `nullslop-tui/src/render.rs`.
pub(crate) fn register(bus: &mut AppBus, _registry: &mut AppUiRegistry) {
    handler::PickerHandler.register(bus);
}
