//! Overlay view registry — slice-registered renderers for dynamic-scope
//! overlays.
//!
//! A slice whose scope is a full-screen capture (the quake bar) draws
//! more than its own cell payload: its rows fold application facts
//! (session lifecycle, prune totals) that live in `AppState`. The
//! [`SliceView`](jinn_slices::view::SliceView) signature passes only the
//! slice payload, so overlay slices register a [`RenderCtx`]-taking
//! closure here instead.
//!
//! Registration happens once, in the slice's `activate()`; the generic
//! render pass resolves the active scope's renderer and draws it. A
//! scope without a registered renderer draws nothing.

use std::collections::HashMap;
use std::sync::Arc;

use derive_more::Debug;
use parking_lot::RwLock;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::common::render_ctx::RenderCtx;
use jinn_slices::SliceScopeId;

/// A slice-registered overlay renderer: draws the slice's overlay into
/// `area` for one frame.
pub type OverlayViewFn = Arc<dyn Fn(&mut Frame<'_>, Rect, &RenderCtx<'_>) + Send + Sync>;

/// The registry of dynamic-scope overlay renderers.
///
/// One entry per overlay slice; written at activation (single writer,
/// read-only at render time). Cheap to clone: shared behind `Arc`.
#[derive(Clone, Debug, Default)]
pub struct OverlayViews {
    #[debug(skip)]
    views: Arc<RwLock<HashMap<SliceScopeId, OverlayViewFn>>>,
}

impl OverlayViews {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers the renderer for `scope`, replacing any previous one.
    pub fn register(&self, scope: SliceScopeId, view: OverlayViewFn) {
        self.views.write().insert(scope, view);
    }

    /// Returns the renderer registered for `scope`, if any.
    #[must_use]
    pub fn view(&self, scope: &SliceScopeId) -> Option<OverlayViewFn> {
        self.views.read().get(scope).cloned()
    }
}
