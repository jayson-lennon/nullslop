//! Character counter UI element.
//!
//! [`CharCounterElement`] renders the grapheme-cluster count of the chat
//! input buffer in the TUI. It is a purely visual element with no command
//! or event handlers.

pub mod element;

use nullslop_plugin_core::Bus;
use nullslop_plugin_ui::UiRegistry;

pub use element::CharCounterElement;

/// Register the character counter UI element.
pub(crate) fn register(_bus: &mut Bus, registry: &mut UiRegistry) {
    registry.register(Box::new(CharCounterElement));
}
