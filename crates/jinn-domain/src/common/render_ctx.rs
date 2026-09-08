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
}

impl<'a> RenderCtx<'a> {
    /// Creates a new render context wrapping the given state reference
    /// and slices registry.
    pub fn new(state: &'a AppState, slices: &'a Slices) -> Self {
        Self { state, slices }
    }
}
