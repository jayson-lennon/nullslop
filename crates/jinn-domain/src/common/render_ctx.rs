//! Standardized render context for the TUI render path.
//!
//! [`RenderCtx`] wraps a shared reference to [`AppState`] plus the
//! slices registry, and is threaded through every render function. It
//! provides a single, extensible context type that can grow to hold
//! command sinks, or other capabilities without changing function
//! signatures. Slice renderers resolve their cells through
//! [`RenderCtx::slices`] instead of reading `FrontendState` fields.

use crate::common::app_state::AppState;
use jinn_slices::Slices;

/// Render context passed to every render function.
///
/// Contains read-only access to application state and the slice
/// registry. Constructed once per frame in the top-level `render()`
/// function and passed through the entire render tree.
pub struct RenderCtx<'a> {
    /// Read-only application state.
    pub state: &'a AppState,
    /// The slices registry: slice renderers resolve read handles here.
    pub slices: &'a Slices,
    /// Slice-registered overlay renderers for dynamic scopes (the quake
    /// bar). Overlay slices register at activation; an unregistered
    /// scope renders nothing.
    pub overlay_views: &'a crate::common::overlay_views::OverlayViews,
}

impl<'a> RenderCtx<'a> {
    /// Creates a new render context wrapping the given state reference,
    /// slices registry, and overlay-view registry.
    pub fn new(
        state: &'a AppState,
        slices: &'a Slices,
        overlay_views: &'a crate::common::overlay_views::OverlayViews,
    ) -> Self {
        Self {
            state,
            slices,
            overlay_views,
        }
    }

    /// Returns the overlay renderer registered for a dynamic scope, if
    /// any. Overlay slices register at activation; an unregistered scope
    /// renders nothing.
    #[must_use]
    pub fn overlay_view(
        &self,
        scope: &jinn_slices::SliceScopeId,
    ) -> Option<crate::common::overlay_views::OverlayViewFn> {
        self.overlay_views.view(scope)
    }
}
