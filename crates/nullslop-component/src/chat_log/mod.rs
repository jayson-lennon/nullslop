//! Chat log UI element.
//!
//! [`ChatLogElement`] renders the chat history in the TUI. It is a purely
//! visual element with no command or event handlers.

pub mod element;

use crate::AppBus;
use crate::AppUiRegistry;

pub use element::ChatLogElement;

/// Register the chat log UI element.
pub(crate) fn register(_bus: &mut AppBus, registry: &mut AppUiRegistry) {
    registry.register(Box::new(ChatLogElement));
}
