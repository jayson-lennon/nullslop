//! Conversation history display.
//!
//! Renders the full scrollable chat log — all messages exchanged in the current
//! session. This is a display-only component — it does not handle any user actions
//! or events.

pub mod element;

use crate::AppBus;
use crate::AppUiRegistry;

pub use element::ChatLogElement;

/// Register the chat log UI element.
pub(crate) fn register(_bus: &mut AppBus, registry: &mut AppUiRegistry) {
    registry.register(Box::new(ChatLogElement));
}
