//! [`UiElement`] trait for renderable UI plugins.
//!
//! Elements draw within an allocated area on the ratatui frame. They are
//! separate from command/event handlers and communicate through
//! [`AppState`] — handlers mutate state during processing, elements
//! read state during rendering.

/// A renderable UI element that draws within an allocated area.
///
/// Elements get full frame access and an allocated area, allowing both
/// constrained rendering (within the given area) and escape-hatch drawing
/// anywhere on the frame if needed.
///
/// UI elements are separate from command/event handlers. They communicate
/// through [`AppState`] — handlers mutate state during processing, elements
/// read state during rendering.
///
/// # Type parameter
///
/// `'static` bound is required for `Box<dyn UiElement>` storage in the
/// [`UiRegistry`](crate::UiRegistry).
pub trait UiElement: 'static + std::fmt::Debug {
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
    fn render(
        &mut self,
        frame: &mut ratatui::Frame<'_>,
        area: ratatui::layout::Rect,
        state: &nullslop_protocol::AppState,
    );
}
