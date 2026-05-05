//! [`UiElement`] trait for renderable UI components.
//!
//! Elements draw within an allocated area on the frame. They are
//! separate from command/event handlers and communicate through
//! [`AppState`] — handlers mutate state during processing, elements
//! read state during rendering.

use ratatui::{Frame, layout::Rect};

/// A renderable UI element that draws within an allocated area.
///
/// Elements get full frame access and an allocated area, allowing both
/// constrained rendering (within the given area) and escape-hatch drawing
/// anywhere on the frame if needed.
///
/// UI elements are separate from command/event handlers. They communicate
/// through state — handlers mutate state during processing, elements
/// read state during rendering.
///
/// # Type parameter
///
/// `'static` bound is required so the element can be stored in the
/// [`UiRegistry`](crate::UiRegistry).
pub trait UiElement<S>: 'static + std::fmt::Debug {
    /// Returns the unique name identifying this element.
    ///
    /// Names are used by the registry for lookup and must be unique
    /// across all registered elements.
    fn name(&self) -> String;

    /// Render the element into the given frame area.
    ///
    /// # Arguments
    ///
    /// * `frame` - Full ratatui frame (elements may draw outside `area` if needed).
    /// * `area` - The allocated region where this element should draw.
    /// * `state` - Read-only application state for rendering decisions.
    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, state: &S);

    /// Returns `true` if this element's content supports text selection.
    ///
    /// When an element returns `true`, the render loop registers its
    /// allocated `Rect` as a selectable region. Mouse clicks and drags
    /// within that rect will trigger application-level selection.
    ///
    /// Default is `false` — most elements don't need selection.
    fn is_selectable(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FakeUiElement;

    #[test]
    fn default_is_selectable_returns_false() {
        // Given a FakeUiElement that does not override is_selectable.
        let (element, _): (FakeUiElement<()>, _) = FakeUiElement::new("test");

        // When calling is_selectable on the trait object.
        let selectable: &dyn UiElement<()> = &element;

        // Then it returns false (the default).
        assert!(!selectable.is_selectable());
    }
}
