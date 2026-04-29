//! Character counter UI element.
//!
//! [`CharCounterElement`] renders the grapheme-cluster count of the chat
//! input buffer in the TUI. It is a purely visual element with no command
//! or event handlers.

pub mod element;

use crate::AppBus;
use crate::AppUiRegistry;

pub use element::CharCounterElement;

/// Register the character counter UI element.
pub(crate) fn register(_bus: &mut AppBus, registry: &mut AppUiRegistry) {
    registry.register(Box::new(CharCounterElement));
}
