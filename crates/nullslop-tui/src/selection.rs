//! Mouse text selection state machine.
//!
//! Tracks the lifecycle of a user's click-and-drag text selection within a
//! constraining rectangular area (typically a UI pane). The state machine has
//! three states: `Idle` (no selection), `Dragging` (mouse button held), and
//! `Active` (selection finalized, awaiting clipboard copy).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use unicode_segmentation::UnicodeSegmentation as _;
use wherror::Error;

/// Screen regions that support text selection, rebuilt each frame.
///
/// During rendering, the layout pushes `Rect`s for selectable areas (chat log,
/// picker popup, etc.). When a mouse click arrives, `find_for_position` returns
/// the most specific (smallest area) matching rect.
#[derive(Debug, Clone, Default)]
pub struct SelectableRects {
    /// The selectable regions.
    rects: Vec<Rect>,
}

impl SelectableRects {
    /// Creates an empty collection.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces all stored rects with a new set.
    pub fn rebuild(&mut self, rects: Vec<Rect>) {
        self.rects = rects;
    }

    /// Returns the smallest rect containing `(x, y)`, or `None`.
    ///
    /// "Smallest" means the rect with the least area — this picks the most
    /// specific pane when rects are nested (e.g. a popup inside the content area).
    /// Ties are broken by first-registered wins (stable iteration order).
    #[must_use]
    pub fn find_for_position(&self, x: u16, y: u16) -> Option<Rect> {
        self.rects
            .iter()
            .filter(|r| {
                x >= r.x && x < r.right() && y >= r.y && y < r.bottom()
            })
            .min_by_key(|r| r.width * r.height)
            .copied()
    }
}

/// Error type for selection-related failures (e.g., clipboard operations).
#[derive(Debug, Error)]
#[error(debug)]
pub struct SelectionError;

/// The state of an in-progress or finalized mouse text selection.
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Default)]
pub enum SelectionState {
    /// No selection in progress.
    #[default]
    Idle,
    /// Mouse drag in progress.
    Dragging {
        /// Position where the drag started (screen coordinates).
        anchor: (u16, u16),
        /// Current drag position (screen coordinates), clamped to bounds.
        focus: (u16, u16),
        /// Constraining rectangle that selection is clipped to.
        bounds: Rect,
    },
    /// Finalized selection awaiting clipboard copy.
    Active {
        /// Position where the drag started (screen coordinates).
        anchor: (u16, u16),
        /// Position where the drag ended (screen coordinates).
        focus: (u16, u16),
        /// Constraining rectangle that selection is clipped to.
        bounds: Rect,
    },
}

impl SelectionState {
    /// Creates a new `Dragging` state starting at the given position.
    ///
    /// Both anchor and focus are initialized to `(x, y)`.
    pub fn start_drag(x: u16, y: u16, bounds: Rect) -> Self {
        Self::Dragging {
            anchor: (x, y),
            focus: (x, y),
            bounds,
        }
    }

    /// Updates the focus position during a drag, clamping to bounds.
    ///
    /// No-op for non-`Dragging` states (returns `self` unchanged).
    #[must_use]
    pub fn update_focus(self, x: u16, y: u16) -> Self {
        match self {
            Self::Dragging {
                anchor,
                bounds,
                ..
            } => {
                let clamped_x = x.clamp(bounds.x, bounds.right().saturating_sub(1));
                let clamped_y = y.clamp(bounds.y, bounds.bottom().saturating_sub(1));
                Self::Dragging {
                    anchor,
                    focus: (clamped_x, clamped_y),
                    bounds,
                }
            }
            other => other,
        }
    }

    /// Finalizes a `Dragging` selection into an `Active` one.
    ///
    /// No-op for non-`Dragging` states.
    #[must_use]
    pub fn finalize(self) -> Self {
        match self {
            Self::Dragging {
                anchor,
                focus,
                bounds,
            } => Self::Active {
                anchor,
                focus,
                bounds,
            },
            other => other,
        }
    }

    /// Cancels any selection, returning to `Idle`.
    #[must_use]
    pub fn cancel(self) -> Self {
        Self::Idle
    }

    /// Returns `true` if the state is anything other than `Idle`.
    pub fn is_active(&self) -> bool {
        !matches!(self, Self::Idle)
    }

    /// Returns the bounding rect of the selection, normalized and clamped.
    ///
    /// The returned rect spans from the top-left corner of the anchor/focus
    /// pair to the bottom-right corner, intersected with the constraining
    /// `bounds`. Returns `None` for `Idle`.
    pub fn selection_rect(&self) -> Option<Rect> {
        match self {
            Self::Idle => None,
            Self::Dragging {
                anchor,
                focus,
                bounds,
            }
            | Self::Active {
                anchor,
                focus,
                bounds,
            } => {
                let x1 = anchor.0.min(focus.0);
                let y1 = anchor.1.min(focus.1);
                let x2 = anchor.0.max(focus.0);
                let y2 = anchor.1.max(focus.1);

                // Clamp to bounds.
                let left = x1.max(bounds.x);
                let top = y1.max(bounds.y);
                let right = x2.min(bounds.right().saturating_sub(1));
                let bottom = y2.min(bounds.bottom().saturating_sub(1));

                if left > right || top > bottom {
                    return None;
                }

                Some(Rect::new(left, top, right - left + 1, bottom - top + 1))
            }
        }
    }

    /// Extracts the selected text from a ratatui buffer.
    ///
    /// Reads characters from the buffer within `selection_rect()`, row by row.
    /// Trailing whitespace is trimmed per row. Rows are joined with `\n`.
    /// Empty trailing rows are omitted. Returns `None` for `Idle`.
    pub fn extract_text(&self, buffer: &Buffer) -> Option<String> {
        let rect = self.selection_rect()?;
        let mut rows: Vec<String> = Vec::new();

        for y in rect.top()..rect.bottom() {
            let mut row_symbols: Vec<String> = Vec::new();
            for x in rect.left()..rect.right() {
                if let Some(cell) = buffer.cell((x, y)) {
                    row_symbols.push(cell.symbol().to_owned());
                }
            }
            let row_text = row_symbols.join("");
            // Trim trailing whitespace per row.
            let trimmed = row_text
                .graphemes(true)
                .collect::<Vec<_>>()
                .iter()
                .rev()
                .skip_while(|g| g.chars().all(char::is_whitespace))
                .copied()
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<String>();
            rows.push(trimmed);
        }

        // Strip empty trailing rows.
        while rows.last().is_some_and(std::string::String::is_empty) {
            rows.pop();
        }

        if rows.is_empty() {
            return Some(String::new());
        }

        Some(rows.join("\n"))
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    fn bounds() -> Rect {
        Rect::new(0, 0, 20, 20)
    }

    #[test]
    fn start_drag_creates_dragging_state() {
        // Given no prior state.
        // When starting a drag at (5, 7) within bounds.
        let state = SelectionState::start_drag(5, 7, bounds());

        // Then the state is Dragging with anchor and focus at (5, 7).
        assert_eq!(
            state,
            SelectionState::Dragging {
                anchor: (5, 7),
                focus: (5, 7),
                bounds: bounds(),
            }
        );
    }

    #[test]
    fn update_focus_clamps_to_bounds() {
        // Given a Dragging state at (5, 5) with bounds (0, 0, 10, 10).
        let state = SelectionState::start_drag(5, 5, Rect::new(0, 0, 10, 10));

        // When updating focus to (15, 15) which exceeds bounds.
        let state = state.update_focus(15, 15);

        // Then the selection rect is clamped to fit within bounds.
        let rect = state.selection_rect().expect("should have a selection rect");
        // Focus clamped to (9, 9), so rect spans (5,5) to (9,9).
        assert_eq!(rect, Rect::new(5, 5, 5, 5));
    }

    #[test]
    fn finalize_transitions_dragging_to_active() {
        // Given a Dragging state.
        let state = SelectionState::start_drag(1, 2, bounds()).update_focus(5, 6);

        // When finalizing.
        let state = state.finalize();

        // Then the state is Active with the same anchor, focus, and bounds.
        assert_eq!(
            state,
            SelectionState::Active {
                anchor: (1, 2),
                focus: (5, 6),
                bounds: bounds(),
            }
        );
    }

    #[test]
    fn cancel_returns_to_idle() {
        // Given a Dragging state.
        let state = SelectionState::start_drag(3, 4, bounds());

        // When cancelling.
        let state = state.cancel();

        // Then the state is Idle.
        assert_eq!(state, SelectionState::Idle);
    }

    #[test]
    fn idle_returns_none_for_selection_rect() {
        // Given an Idle state.
        let state = SelectionState::Idle;

        // When asking for selection_rect.
        // Then it returns None.
        assert!(state.selection_rect().is_none());
    }

    #[test]
    fn idle_returns_none_for_extract_text() {
        // Given an Idle state and an empty buffer.
        let state = SelectionState::Idle;
        let buffer = Buffer::empty(Rect::new(0, 0, 10, 5));

        // When extracting text.
        // Then it returns None.
        assert!(state.extract_text(&buffer).is_none());
    }

    #[test]
    fn extract_text_reads_single_row() {
        // Given a buffer with known text on row 2.
        let area = Rect::new(0, 0, 10, 5);
        let mut buffer = Buffer::empty(area);
        // Write "Hello" starting at (2, 2).
        for (i, ch) in "Hello".chars().enumerate() {
            let cell = buffer.cell_mut((2 + i as u16, 2)).unwrap();
            cell.set_symbol(&ch.to_string());
        }

        // And an Active selection covering cells (2,2) to (6,2).
        let state = SelectionState::Active {
            anchor: (2, 2),
            focus: (6, 2),
            bounds: area,
        };

        // When extracting text.
        let text = state.extract_text(&buffer).expect("should return text");

        // Then the text matches the content of those cells.
        assert_eq!(text, "Hello");
    }

    #[test]
    fn extract_text_reads_multiple_rows() {
        // Given a buffer with text on two rows.
        let area = Rect::new(0, 0, 10, 5);
        let mut buffer = Buffer::empty(area);
        // Row 1: "AB" at (0, 1) and (1, 1).
        buffer.cell_mut((0, 1)).unwrap().set_symbol("A");
        buffer.cell_mut((1, 1)).unwrap().set_symbol("B");
        // Row 2: "CD" at (0, 2) and (1, 2).
        buffer.cell_mut((0, 2)).unwrap().set_symbol("C");
        buffer.cell_mut((1, 2)).unwrap().set_symbol("D");

        // And an Active selection spanning rows 1 and 2.
        let state = SelectionState::Active {
            anchor: (0, 1),
            focus: (1, 2),
            bounds: area,
        };

        // When extracting text.
        let text = state.extract_text(&buffer).expect("should return text");

        // Then the rows are joined with newline.
        assert_eq!(text, "AB\nCD");
    }

    #[test]
    fn selection_rect_anchor_can_be_after_focus() {
        // Given an Active state where anchor (5, 5) is after focus (2, 2).
        let state = SelectionState::Active {
            anchor: (5, 5),
            focus: (2, 2),
            bounds: bounds(),
        };

        // When asking for selection_rect.
        let rect = state.selection_rect().expect("should have a selection rect");

        // Then the rect is normalized to top-left origin (2, 2).
        assert_eq!(rect, Rect::new(2, 2, 4, 4)); // (2,2) to (5,5) = width 4, height 4
    }

    // --- SelectableRects tests ---

    #[test]
    fn selectable_rects_find_returns_smallest_matching() {
        // Given overlapping rects — a large screen and a smaller pane.
        let screen = Rect::new(0, 0, 80, 24);
        let pane = Rect::new(10, 5, 20, 10);
        let mut rects = SelectableRects::new();
        rects.rebuild(vec![screen, pane]);

        // When querying a position inside the pane.
        let found = rects.find_for_position(15, 8);

        // Then the smaller pane rect is returned.
        assert_eq!(found, Some(pane));
    }

    #[test]
    fn selectable_rects_find_returns_none_for_position_outside_all() {
        // Given a single rect.
        let mut rects = SelectableRects::new();
        rects.rebuild(vec![Rect::new(0, 0, 10, 10)]);

        // When querying a position outside the rect.
        let found = rects.find_for_position(20, 20);

        // Then None is returned.
        assert_eq!(found, None);
    }

    #[test]
    fn selectable_rects_find_returns_none_when_empty() {
        // Given no rects registered.
        let rects = SelectableRects::new();

        // When querying any position.
        let found = rects.find_for_position(5, 5);

        // Then None is returned.
        assert_eq!(found, None);
    }

    #[test]
    fn selectable_rects_rebuild_replaces_previous_rects() {
        // Given rects with an initial rect.
        let mut rects = SelectableRects::new();
        rects.rebuild(vec![Rect::new(0, 0, 10, 10)]);

        // When rebuilding with different rects.
        rects.rebuild(vec![Rect::new(20, 20, 5, 5)]);

        // Then the old rect is gone and only the new one matches.
        assert_eq!(rects.find_for_position(5, 5), None);
        assert_eq!(rects.find_for_position(22, 22), Some(Rect::new(20, 20, 5, 5)));
    }
}
