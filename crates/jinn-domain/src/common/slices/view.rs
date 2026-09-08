//! Generic slice views and the viewport that pairs them with cells.
//!
//! A *view* is the third artifact of the contribution triple (STATE =
//! slice cell, LOGIC = owning actor, VIEW = this): a pure renderer that
//! draws one slice's payload. Views are generic over their payload, so
//! a `DashboardView` writes `&DashboardState` and nothing looser. The
//! [`Viewport`] stores views behind the [`ErasedView`] twin (the same
//! trick as the UI registry) and re-types them at render time by
//! resolving the view's slot against the [`Slices`] registry.
//!
//! The re-typing happens once, in
//! [`Viewport::register`](Viewport::register) — not per frame, and not
//! scattered across render code: if a view's slot is missing or its
//! cell's payload type doesn't match, registration fails at startup.

use std::fmt;

use jinn_theme::Theme;
use ratatui::Frame;
use ratatui::layout::Rect;

use super::Slices;
use super::SlotKey;

/// Per-frame inputs a view needs beyond its slice.
///
/// Application data that is not slice payload but every view draws
/// with — currently the resolved theme. Grows deliberately: anything
/// added here must be cheap to borrow and immutable for the frame.
#[derive(Debug)]
pub struct ViewCx<'a> {
    /// The application's resolved theme for this frame.
    pub theme: &'a Theme,
}

/// A pure renderer for one slice's payload.
///
/// Implementations own no domain state — everything they draw comes from
/// `&Self::Slice` (plus view-local display state such as cached table
/// widgets). `render` must not mutate the slice or reach back into
/// `AppState`.
pub trait SliceView: fmt::Debug {
    /// The payload type this view draws.
    type Slice: Send + Sync + 'static;

    /// The slot this view renders.
    fn slot(&self) -> SlotKey;

    /// Draws the slice into `area`.
    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, cx: &ViewCx<'_>, slice: &Self::Slice);
}

/// Type-erased view stored by the [`Viewport`].
///
/// The erased twin keeps the viewport a homogeneous `Vec` while the
/// generic [`SliceView`] does the actual drawing. `render_erased`
/// resolves the view's slot to a cell of *its* slice type; a
/// resolution failure here is a programming error guarded against at
/// `register` time, so the `Option` is an internal invariant, not an
/// error path callers handle.
pub trait ErasedView: fmt::Debug + Send + Sync {
    /// The slot this view renders.
    fn slot(&self) -> SlotKey;

    /// Draws the view after re-typing its slice from `slices`.
    fn render_erased(
        &mut self,
        frame: &mut Frame<'_>,
        area: Rect,
        cx: &ViewCx<'_>,
        slices: &Slices,
    );
}

/// Adapter: erases any [`SliceView`] while preserving its typing.
#[derive(derive_more::Debug)]
struct ViewAdapter<V>
where
    V: SliceView,
{
    view: V,
}

impl<V> ErasedView for ViewAdapter<V>
where
    V: SliceView + Send + Sync,
{
    fn slot(&self) -> SlotKey {
        self.view.slot()
    }

    fn render_erased(
        &mut self,
        frame: &mut Frame<'_>,
        area: Rect,
        cx: &ViewCx<'_>,
        slices: &Slices,
    ) {
        let Some(cell) = slices.reader::<V::Slice>(&self.view.slot()) else {
            // Unreachable for views that went through `Viewport::register`;
            // cells are never unregistered, so the pairing holds for life.
            return;
        };
        let guard = cell.read();
        self.view.render(frame, area, cx, &guard);
    }
}

/// View/slot pairing rejected at startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewSlotError {
    /// The view's slot that failed to resolve.
    pub key: SlotKey,
    /// Why the pairing failed.
    pub reason: ViewSlotErrorReason,
}

impl fmt::Display for ViewSlotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "view slot {} ", self.key)?;
        match self.reason {
            ViewSlotErrorReason::Unregistered => write!(f, "is not registered"),
            ViewSlotErrorReason::TypeMismatch { expected, actual } => {
                write!(f, "holds {actual}, expected {expected}")
            }
        }
    }
}

impl std::error::Error for ViewSlotError {}

/// Why a view/slot pairing failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewSlotErrorReason {
    /// No cell exists under the slot.
    Unregistered,
    /// A cell exists, but its payload is a different type.
    TypeMismatch {
        /// The view's slice type.
        expected: &'static str,
        /// The cell's actual payload type.
        actual: &'static str,
    },
}

/// Holds the views that draw slices, keyed by their slots.
///
/// Views are registered during startup — pairing each view with its
/// cell is verified there, so a mismatch aborts launch instead of
/// corrupting frame 400. Registration order is preserved; slots are
/// unique per viewport.
#[derive(Debug, Default)]
pub struct Viewport {
    views: Vec<Box<dyn ErasedView>>,
}

impl Clone for Viewport {
    fn clone(&self) -> Self {
        Self {
            // `Services: Clone` demands it, but views are registered once
            // at bootstrap; a clone is an empty shell, never a usable
            // renderer. Documented loudly so nobody renders a clone.
            views: Vec::new(),
        }
    }
}

impl Viewport {
    /// Creates an empty viewport.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `view` and asserts its slot resolves to a cell of the
    /// view's slice type.
    ///
    /// This is the startup pairing assertion: it reads through
    /// [`Slices::reader`] immediately, so a missing cell or a wrong
    /// payload type fails here, at launch, rather than silently blank
    /// during rendering.
    ///
    /// # Errors
    ///
    /// Returns [`ViewSlotError`] if the view's slot is unregistered or
    /// its cell's payload type differs from the view's slice type.
    pub fn register<V>(&mut self, view: V, slices: &Slices) -> Result<(), ViewSlotError>
    where
        V: SliceView + Send + Sync + 'static,
    {
        let key = view.slot();
        let expected = std::any::type_name::<V::Slice>();
        if let Some(actual) = slices.slot_type(&key) {
            if slices.reader::<V::Slice>(&key).is_none() {
                return Err(ViewSlotError {
                    key,
                    reason: ViewSlotErrorReason::TypeMismatch { expected, actual },
                });
            }
        } else {
            return Err(ViewSlotError {
                key,
                reason: ViewSlotErrorReason::Unregistered,
            });
        }
        self.views.push(Box::new(ViewAdapter { view }));
        Ok(())
    }

    /// Renders the view registered for `key`, if any.
    ///
    /// Unregistered keys render nothing — the caller decides whether a
    /// hidden tab is an error or simply not shown.
    pub fn render_slot(
        &mut self,
        frame: &mut Frame<'_>,
        area: Rect,
        key: &SlotKey,
        cx: &ViewCx<'_>,
        slices: &Slices,
    ) {
        for view in &mut self.views {
            if view.slot() == *key {
                view.render_erased(frame, area, cx, slices);
                return;
            }
        }
    }
}
