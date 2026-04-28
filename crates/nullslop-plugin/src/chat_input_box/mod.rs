//! Chat input box plugin: consolidated handler and UI element.
//!
//! This module provides a single [`ChatInputBoxHandler`] that handles all
//! chat input and mode-switching commands (previously split across
//! `InputModePlugin` and `NormalModePlugin`), and a [`ChatInputBoxElement`]
//! that renders the input box in the TUI.
//!
//! The [`ChatInputBoxState`] (defined in `nullslop-protocol`) encapsulates
//! the input buffer and is re-exported here for co-location with the plugin.

pub mod element;
pub mod handler;

use nullslop_plugin_core::Bus;
use nullslop_plugin_ui::UiRegistry;

pub use npr::ChatInputBoxState;
pub use element::ChatInputBoxElement;
pub(crate) use handler::ChatInputBoxHandler;

use nullslop_protocol as npr;

/// Register the chat input box handler and UI element.
pub(crate) fn register(bus: &mut Bus, registry: &mut UiRegistry) {
    ChatInputBoxHandler.register(bus);
    registry.register(Box::new(ChatInputBoxElement));
}
