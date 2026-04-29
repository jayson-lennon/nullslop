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

use crate::AppBus;
use crate::AppUiRegistry;

pub use crate::ChatInputBoxState;
pub use element::ChatInputBoxElement;
pub(crate) use handler::ChatInputBoxHandler;

/// Register the chat input box handler and UI element.
pub(crate) fn register(bus: &mut AppBus, registry: &mut AppUiRegistry) {
    ChatInputBoxHandler.register(bus);
    registry.register(Box::new(ChatInputBoxElement));
}
