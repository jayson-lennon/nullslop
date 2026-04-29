//! Chat input box — where the user composes and sends messages.
//!
//! This component manages the text input experience end to end: handling keystrokes,
//! displaying the in-progress message, tracking the input buffer, and switching
//! between browsing and typing modes.

pub mod element;
pub mod handler;
pub mod state;

use crate::AppBus;
use crate::AppUiRegistry;

pub use element::ChatInputBoxElement;
pub(crate) use handler::ChatInputBoxHandler;
pub use state::ChatInputBoxState;

/// Register the chat input box handler and UI element.
pub(crate) fn register(bus: &mut AppBus, registry: &mut AppUiRegistry) {
    ChatInputBoxHandler.register(bus);
    registry.register(Box::new(ChatInputBoxElement));
}
