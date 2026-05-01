//! Conversation history display.
//!
//! Renders the full scrollable chat log — all messages exchanged in the current
//! session. Also handles `PushChatEntry` commands to record entries in history
//! and emit `ChatEntrySubmitted` events.

pub mod element;
mod handler;

use crate::AppBus;
use crate::AppUiRegistry;

pub use element::ChatLogElement;
pub(crate) use handler::ChatLogHandler;

/// Register the chat log UI element and handler.
pub(crate) fn register(bus: &mut AppBus, registry: &mut AppUiRegistry) {
    ChatLogHandler.register(bus);
    registry.register(Box::new(ChatLogElement));
}
