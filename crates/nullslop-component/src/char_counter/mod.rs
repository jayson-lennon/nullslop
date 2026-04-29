//! Character counter display.
//!
//! Shows a live count of how many characters the user has typed in the input box.
//! This is a display-only component — it does not handle any user actions or events.

pub mod element;

use crate::AppBus;
use crate::AppUiRegistry;

pub use element::CharCounterElement;

/// Register the character counter UI element.
pub(crate) fn register(_bus: &mut AppBus, registry: &mut AppUiRegistry) {
    registry.register(Box::new(CharCounterElement));
}
