//! Picker state — filter text, cursor position, and selection index for the provider picker overlay.

use unicode_segmentation::UnicodeSegmentation as _;

/// State for the provider picker overlay.
///
/// Holds the current filter text, cursor position within that text,
/// and the index of the highlighted item in the filtered list.
/// Tracks a scroll offset for windowed result display.
/// Reset when the picker opens.
#[derive(Debug, Clone, Default)]
pub struct ProviderPickerState {
    /// Current filter text.
    pub filter: String,
    /// Index of the currently highlighted item in the filtered list.
    pub selection: usize,
    /// Cursor position as a grapheme-cluster index within `filter` (0 = before first grapheme).
    pub cursor_pos: usize,
    /// Index of the first visible result row (scroll window top).
    pub scroll_offset: usize,
}

impl ProviderPickerState {
    /// Creates a new, empty picker state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a character at the cursor position, advancing the cursor and resetting selection.
    pub fn insert_char(&mut self, ch: char) {
        let byte_offset = self
            .filter
            .grapheme_indices(true)
            .nth(self.cursor_pos)
            .map_or(self.filter.len(), |(i, _)| i);
        self.filter.insert(byte_offset, ch);
        self.cursor_pos += 1;
        self.selection = 0;
        self.scroll_offset = 0;
    }

    /// Deletes the grapheme before the cursor, decrementing the cursor and resetting selection.
    ///
    /// No-op when the cursor is at position 0.
    #[expect(
        clippy::indexing_slicing,
        reason = "delete_idx is cursor_pos - 1 where cursor_pos > 0, and graphemes length equals grapheme count which is >= cursor_pos"
    )]
    pub fn backspace(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }
        let graphemes: Vec<(usize, &str)> = self.filter.grapheme_indices(true).collect();
        let delete_idx = self.cursor_pos - 1;
        let (start, g) = graphemes[delete_idx];
        let end = start + g.len();
        self.filter.drain(start..end);
        self.cursor_pos -= 1;
        self.selection = 0;
        self.scroll_offset = 0;
    }

    /// Moves the cursor one grapheme to the left.
    ///
    /// No-op when the cursor is already at position 0.
    pub fn move_cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    /// Moves the cursor one grapheme to the right.
    ///
    /// No-op when the cursor is already at the end of the filter text.
    pub fn move_cursor_right(&mut self) {
        if self.cursor_pos < self.filter.graphemes(true).count() {
            self.cursor_pos += 1;
        }
    }

    /// Returns the current cursor position as a grapheme index.
    #[must_use]
    pub fn cursor_pos(&self) -> usize {
        self.cursor_pos
    }

    /// Returns the current scroll offset (index of first visible result row).
    #[must_use]
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Adjusts `scroll_offset` so that `selection` is within the visible window.
    pub fn ensure_visible(&mut self, max_visible: usize) {
        if self.selection < self.scroll_offset {
            self.scroll_offset = self.selection;
        } else if max_visible > 0 && self.selection >= self.scroll_offset + max_visible {
            self.scroll_offset = self.selection - max_visible + 1;
        } else {
            // Selection is within the visible window — no adjustment needed.
        }
    }

    /// Moves the selection up, clamping at 0, then adjusts scroll offset.
    pub fn move_up(&mut self, _max: usize, max_visible: usize) {
        if self.selection > 0 {
            self.selection -= 1;
        }
        self.ensure_visible(max_visible);
    }

    /// Moves the selection down, clamping at `max - 1`, then adjusts scroll offset.
    pub fn move_down(&mut self, max: usize, max_visible: usize) {
        if max > 0 && self.selection < max - 1 {
            self.selection += 1;
        }
        self.ensure_visible(max_visible);
    }

    /// Resets the picker state (called when opening).
    pub fn reset(&mut self) {
        self.filter.clear();
        self.selection = 0;
        self.cursor_pos = 0;
        self.scroll_offset = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- insert_char tests ---

    #[test]
    fn insert_char_appends_to_filter() {
        // Given a fresh picker state.
        let mut state = ProviderPickerState::new();

        // When inserting 'a' then 'b'.
        state.insert_char('a');
        state.insert_char('b');

        // Then the filter is "ab".
        assert_eq!(state.filter, "ab");
    }

    #[test]
    fn insert_char_resets_selection() {
        // Given a picker state with selection at 3 and scroll_offset at 2.
        let mut state = ProviderPickerState::new();
        state.selection = 3;
        state.scroll_offset = 2;

        // When inserting a character.
        state.insert_char('x');

        // Then selection and scroll_offset are reset to 0.
        assert_eq!(state.selection, 0);
        assert_eq!(state.scroll_offset, 0);
    }

    #[test]
    fn insert_char_at_cursor_middle() {
        // Given a picker state with filter "abc" and cursor at 1.
        let mut state = ProviderPickerState::new();
        state.filter = "abc".to_owned();
        state.cursor_pos = 1;

        // When inserting 'x' at cursor.
        state.insert_char('x');

        // Then the filter is "axbc" and cursor advanced to 2.
        assert_eq!(state.filter, "axbc");
        assert_eq!(state.cursor_pos(), 2);
    }

    #[test]
    fn insert_char_advances_cursor() {
        // Given a fresh picker state.
        let mut state = ProviderPickerState::new();

        // When inserting 'a'.
        state.insert_char('a');

        // Then cursor is at 1.
        assert_eq!(state.cursor_pos(), 1);
    }

    // --- backspace tests ---

    #[test]
    fn backspace_removes_before_cursor() {
        // Given a picker state with filter "ab" and cursor at end (position 2).
        let mut state = ProviderPickerState::new();
        state.filter = "ab".to_owned();
        state.cursor_pos = 2;

        // When pressing backspace.
        state.backspace();

        // Then the filter is "a" and cursor is at 1.
        assert_eq!(state.filter, "a");
        assert_eq!(state.cursor_pos(), 1);
    }

    #[test]
    fn backspace_resets_selection() {
        // Given a picker state with filter "ab", cursor at end (2), selection at 3, and scroll_offset at 2.
        let mut state = ProviderPickerState::new();
        state.filter = "ab".to_owned();
        state.cursor_pos = 2;
        state.selection = 3;
        state.scroll_offset = 2;

        // When pressing backspace.
        state.backspace();

        // Then selection and scroll_offset are reset to 0.
        assert_eq!(state.selection, 0);
        assert_eq!(state.scroll_offset, 0);
    }

    #[test]
    fn backspace_at_cursor_middle() {
        // Given a picker state with filter "abc" and cursor at 2.
        let mut state = ProviderPickerState::new();
        state.filter = "abc".to_owned();
        state.cursor_pos = 2;

        // When pressing backspace.
        state.backspace();

        // Then the filter is "ac" and cursor is at 1.
        assert_eq!(state.filter, "ac");
        assert_eq!(state.cursor_pos(), 1);
    }

    #[test]
    fn backspace_at_start_is_noop() {
        // Given a picker state with filter "abc" and cursor at 0.
        let mut state = ProviderPickerState::new();
        state.filter = "abc".to_owned();
        state.cursor_pos = 0;

        // When pressing backspace.
        state.backspace();

        // Then filter is unchanged and cursor is still 0.
        assert_eq!(state.filter, "abc");
        assert_eq!(state.cursor_pos(), 0);
    }

    // --- cursor movement tests ---

    #[test]
    fn move_cursor_left_decrements() {
        // Given a picker state with cursor at 3.
        let mut state = ProviderPickerState::new();
        state.cursor_pos = 3;

        // When moving cursor left.
        state.move_cursor_left();

        // Then cursor is at 2.
        assert_eq!(state.cursor_pos(), 2);
    }

    #[test]
    fn move_cursor_left_clamps_at_zero() {
        // Given a picker state with cursor at 0.
        let mut state = ProviderPickerState::new();

        // When moving cursor left.
        state.move_cursor_left();

        // Then cursor stays at 0.
        assert_eq!(state.cursor_pos(), 0);
    }

    #[test]
    fn move_cursor_right_increments() {
        // Given a picker state with filter "abc" and cursor at 1.
        let mut state = ProviderPickerState::new();
        state.filter = "abc".to_owned();
        state.cursor_pos = 1;

        // When moving cursor right.
        state.move_cursor_right();

        // Then cursor is at 2.
        assert_eq!(state.cursor_pos(), 2);
    }

    #[test]
    fn move_cursor_right_clamps_at_end() {
        // Given a picker state with filter "abc" and cursor at 3 (end).
        let mut state = ProviderPickerState::new();
        state.filter = "abc".to_owned();
        state.cursor_pos = 3;

        // When moving cursor right.
        state.move_cursor_right();

        // Then cursor stays at 3.
        assert_eq!(state.cursor_pos(), 3);
    }

    // --- selection movement tests ---

    #[test]
    fn move_up_decrements() {
        // Given a picker state with selection at 3.
        let mut state = ProviderPickerState::new();
        state.selection = 3;

        // When moving up with max 5.
        state.move_up(5, 5);

        // Then selection is 2.
        assert_eq!(state.selection, 2);
    }

    #[test]
    fn move_up_clamps_at_zero() {
        // Given a picker state with selection at 0.
        let mut state = ProviderPickerState::new();

        // When moving up.
        state.move_up(5, 5);

        // Then selection stays at 0.
        assert_eq!(state.selection, 0);
    }

    #[test]
    fn move_down_increments() {
        // Given a picker state with selection at 1.
        let mut state = ProviderPickerState::new();
        state.selection = 1;

        // When moving down with max 5.
        state.move_down(5, 5);

        // Then selection is 2.
        assert_eq!(state.selection, 2);
    }

    #[test]
    fn move_down_clamps_at_max() {
        // Given a picker state with selection at 4, max 5.
        let mut state = ProviderPickerState::new();
        state.selection = 4;

        // When moving down.
        state.move_down(5, 5);

        // Then selection stays at 4.
        assert_eq!(state.selection, 4);
    }

    #[test]
    fn move_down_clamps_when_empty() {
        // Given a picker state with selection at 0, max 0.
        let mut state = ProviderPickerState::new();

        // When moving down.
        state.move_down(0, 5);

        // Then selection stays at 0.
        assert_eq!(state.selection, 0);
    }

    // --- scroll offset tests ---

    #[test]
    fn move_up_adjusts_scroll_offset_when_selection_above_view() {
        // Given a picker state with scroll_offset=2 and selection=2.
        let mut state = ProviderPickerState::new();
        state.selection = 2;
        state.scroll_offset = 2;

        // When moving up with max_visible=5.
        state.move_up(10, 5);

        // Then selection is 1 and scroll_offset adjusts to 1.
        assert_eq!(state.selection, 1);
        assert_eq!(state.scroll_offset, 1);
    }

    #[test]
    fn move_down_adjusts_scroll_offset_when_selection_below_view() {
        // Given a picker state with scroll_offset=0, selection=4, max_visible=5.
        let mut state = ProviderPickerState::new();
        state.selection = 4;
        state.scroll_offset = 0;

        // When moving down with max=10, max_visible=5.
        state.move_down(10, 5);

        // Then selection is 5 and scroll_offset adjusts to 1.
        assert_eq!(state.selection, 5);
        assert_eq!(state.scroll_offset, 1);
    }

    #[test]
    fn ensure_visible_selection_within_view() {
        // Given a picker state with scroll_offset=2 and selection=3.
        let mut state = ProviderPickerState::new();
        state.scroll_offset = 2;
        state.selection = 3;

        // When ensuring visible with max_visible=5.
        state.ensure_visible(5);

        // Then scroll_offset stays at 2 (3 is within [2, 7)).
        assert_eq!(state.scroll_offset, 2);
    }

    #[test]
    fn ensure_visible_selection_above_view() {
        // Given a picker state with scroll_offset=3 and selection=1.
        let mut state = ProviderPickerState::new();
        state.scroll_offset = 3;
        state.selection = 1;

        // When ensuring visible with max_visible=5.
        state.ensure_visible(5);

        // Then scroll_offset adjusts up to 1.
        assert_eq!(state.scroll_offset, 1);
    }

    #[test]
    fn ensure_visible_selection_below_view() {
        // Given a picker state with scroll_offset=0 and selection=7.
        let mut state = ProviderPickerState::new();
        state.scroll_offset = 0;
        state.selection = 7;

        // When ensuring visible with max_visible=5.
        state.ensure_visible(5);

        // Then scroll_offset adjusts down to 3.
        assert_eq!(state.scroll_offset, 3);
    }

    // --- reset tests ---

    #[test]
    fn reset_clears_everything() {
        // Given a picker state with filter "ab", selection 3, cursor at 2, scroll_offset 1.
        let mut state = ProviderPickerState::new();
        state.filter = "ab".to_owned();
        state.selection = 3;
        state.cursor_pos = 2;
        state.scroll_offset = 1;

        // When resetting.
        state.reset();

        // Then filter is empty, selection is 0, cursor is 0, scroll_offset is 0.
        assert!(state.filter.is_empty());
        assert_eq!(state.selection, 0);
        assert_eq!(state.cursor_pos(), 0);
        assert_eq!(state.scroll_offset, 0);
    }
}
