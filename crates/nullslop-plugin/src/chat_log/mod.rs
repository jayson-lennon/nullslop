//! Chat log UI element.
//!
//! [`ChatLogElement`] renders the chat history in the TUI. It is a purely
//! visual element with no command or event handlers.

pub mod element;

use nullslop_plugin_core::Bus;
use nullslop_plugin_ui::UiRegistry;

pub use element::ChatLogElement;

/// Register the chat log UI element.
pub(crate) fn register(_bus: &mut Bus, registry: &mut UiRegistry) {
    registry.register(Box::new(ChatLogElement));
}
