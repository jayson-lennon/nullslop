//! Chat input box component: consolidated handler and UI element.
//!
//! This module provides a single [`ChatInputBoxHandler`] that handles all
//! chat input and mode-switching commands (previously split across
//! `InputModeComponent` and `NormalModeComponent`), and a [`ChatInputBoxElement`]
//! that renders the input box in the TUI.
//!
//! The [`ChatInputBoxState`] (defined in `nullslop-component-core`) encapsulates
//! the input buffer and is re-exported here for co-location with the component.

pub mod element;
pub mod handler;

use nullslop_component_core::Bus;
use nullslop_component_ui::UiRegistry;

pub use element::ChatInputBoxElement;
pub(crate) use handler::ChatInputBoxHandler;
pub use nullslop_component_core::ChatInputBoxState;

/// Register the chat input box handler and UI element.
pub(crate) fn register(bus: &mut Bus, registry: &mut UiRegistry) {
    ChatInputBoxHandler.register(bus);
    registry.register(Box::new(ChatInputBoxElement));
}
