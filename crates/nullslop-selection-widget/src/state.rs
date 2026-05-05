//! Selection state — the core state machine for the picker widget.
//!
//! [`SelectionState`] holds the filter text, cursor position, selection index, scroll offset,
//! the full item list, and a cached filtered index list. Filter input methods trigger
//! fuzzy re-filtering and reset the selection. Navigation methods move the selection
//! within the filtered results and adjust the scroll window.

use fuzzy_matcher::FuzzyMatcher as _;
use fuzzy_matcher::skim::SkimMatcherV2;
use unicode_segmentation::UnicodeSegmentation as _;

use crate::PickerItem;

/// State machine for a search+filter+select picker.
///
/// Generic over any type implementing [`PickerItem`]. Owns the item list and caches
/// filtered results as **indices** into that list (no cloning on every keystroke).
///
/// # Examples
///
/// ```ignore
/// let mut state = SelectionState::new();
/// state.set_items(vec![MyItem::new("hello"), MyItem::new("world")]);
/// state.insert_char('h');
/// assert_eq!(state.filtered_count(), 1);
/// ```
#[derive(Debug)]
pub struct SelectionState<T: PickerItem> {
    /// Current filter text typed by the user.
    filter: String,
    /// Cursor position as a grapheme-cluster index within `filter` (0 = before first grapheme).
    cursor_pos: usize,
    /// Index of the currently highlighted item in the filtered list.
    selection: usize,
    /// Index of the first visible result row (scroll window top).
    scroll_offset: usize,
    /// The full item list provided by the consumer (pre-sorted).
    items: Vec<T>,
    /// Cached indices into `items` for matching entries, recomputed on filter change.
    filtered_indices: Vec<usize>,
}

impl<T: PickerItem> Default for SelectionState<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: PickerItem> SelectionState<T> {
    /// Creates a new, empty selection state with no items.
    #[must_use]
    pub fn new() -> Self {
        Self {
            filter: String::new(),
            cursor_pos: 0,
            selection: 0,
            scroll_offset: 0,
            items: Vec::new(),
            filtered_indices: Vec::new(),
        }
    }

    /// Creates a selection state pre-populated with items.
    ///
    /// When the filter is empty (which it is initially), all items are visible.
    #[must_use]
    pub fn with_items(items: Vec<T>) -> Self {
        let filtered_indices = (0..items.len()).collect();
        Self {
            filter: String::new(),
            cursor_pos: 0,
            selection: 0,
            scroll_offset: 0,
            items,
            filtered_indices,
        }
    }

    // --- Item management ---

    /// Replaces the full item list and re-filters against the current filter text.
    ///
    /// Does **not** reset the filter text or cursor position — the consumer may want to
    /// update items while the picker is open (e.g., after a model cache refresh).
    /// Clamps `selection` to stay within the new filtered bounds.
    pub fn set_items(&mut self, items: Vec<T>) {
        self.items = items;
        self.recompute_filtered();
        self.selection = self
            .selection
            .min(self.filtered_indices.len().saturating_sub(1));
    }

    // --- Filter input methods (all trigger re-filter, reset selection to 0) ---

    /// Inserts a character at the cursor position, advances the cursor, re-filters, and
    /// resets selection and scroll offset to 0.
    pub fn insert_char(&mut self, ch: char) {
        let byte_offset = self
            .filter
            .grapheme_indices(true)
            .nth(self.cursor_pos)
            .map_or(self.filter.len(), |(i, _)| i);
        self.filter.insert(byte_offset, ch);
        self.cursor_pos += 1;
        self.recompute_filtered();
        self.selection = 0;
        self.scroll_offset = 0;
    }

    /// Deletes the grapheme before the cursor, decrements the cursor, re-filters, and
    /// resets selection and scroll offset to 0.
    ///
    /// No-op when the cursor is at position 0.
    pub fn backspace(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }
        let graphemes: Vec<_> = self.filter.grapheme_indices(true).collect();
        let delete_idx = self.cursor_pos - 1;
        let Some(&(start, g)) = graphemes.get(delete_idx) else {
            return;
        };
        let end = start + g.len();
        self.filter.drain(start..end);
        self.cursor_pos -= 1;
        self.recompute_filtered();
        self.selection = 0;
        self.scroll_offset = 0;
    }

    // --- Cursor movement (do NOT trigger re-filter or reset selection) ---

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

    // --- Selection movement (do NOT trigger re-filter) ---

    /// Moves the selection up by one, clamping at 0, then adjusts scroll offset.
    ///
    /// `max_visible` is the number of visible rows in the picker area, used to keep
    /// the selection within the scroll window.
    pub fn move_up(&mut self, max_visible: usize) {
        if self.selection > 0 {
            self.selection -= 1;
        }
        self.ensure_visible(max_visible);
    }

    /// Moves the selection down by one, clamping at the end of the filtered list,
    /// then adjusts scroll offset.
    ///
    /// `max_visible` is the number of visible rows in the picker area, used to keep
    /// the selection within the scroll window.
    pub fn move_down(&mut self, max_visible: usize) {
        let max = self.filtered_indices.len();
        if max > 0 && self.selection < max - 1 {
            self.selection += 1;
        }
        self.ensure_visible(max_visible);
    }

    // --- Scroll ---

    /// Adjusts `scroll_offset` so that `selection` is within the visible window.
    ///
    /// `max_visible` is the number of rows that fit in the picker area.
    pub fn ensure_visible(&mut self, max_visible: usize) {
        if self.selection < self.scroll_offset {
            self.scroll_offset = self.selection;
        } else if max_visible > 0 && self.selection >= self.scroll_offset + max_visible {
            self.scroll_offset = self.selection - max_visible + 1;
        } else {
            // Selection is within the visible window — no adjustment needed.
        }
    }

    // --- Reset ---

    /// Clears the filter text and resets selection, cursor, and scroll offset to 0.
    ///
    /// Does **not** clear the item list — the consumer manages the item lifecycle.
    /// The filtered list is recomputed (all items are visible when filter is empty).
    pub fn reset(&mut self) {
        self.filter.clear();
        self.selection = 0;
        self.cursor_pos = 0;
        self.scroll_offset = 0;
        self.recompute_filtered();
    }

    // --- Read access ---

    /// Returns the current filter text.
    #[must_use]
    pub fn filter(&self) -> &str {
        &self.filter
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

    /// Returns the current selection index within the filtered list.
    #[must_use]
    pub fn selection(&self) -> usize {
        self.selection
    }

    /// Sets the selection index directly.
    ///
    /// Clamps to the valid range `[0, filtered_count - 1]`.
    /// Primarily for test setup — production code should use [`move_up`](Self::move_up)
    /// and [`move_down`](Self::move_down).
    pub fn set_selection(&mut self, idx: usize) {
        let max = self.filtered_indices.len();
        self.selection = if max > 0 { idx.min(max - 1) } else { 0 };
    }

    /// Returns the currently selected item, or `None` if the filtered list is empty.
    #[must_use]
    pub fn selected_item(&self) -> Option<&T> {
        let &i = self.filtered_indices.get(self.selection)?;
        self.items.get(i)
    }

    /// Returns the number of items in the filtered list.
    #[must_use]
    pub fn filtered_count(&self) -> usize {
        self.filtered_indices.len()
    }

    /// Returns the filtered item at the given index, or `None` if out of bounds.
    #[must_use]
    pub fn filtered_item(&self, idx: usize) -> Option<&T> {
        let &i = self.filtered_indices.get(idx)?;
        self.items.get(i)
    }

    /// Returns the full item list (all items, not just filtered).
    #[must_use]
    pub fn items(&self) -> &[T] {
        &self.items
    }

    // --- Internal ---

    /// Recomputes the filtered index cache based on the current filter text.
    ///
    /// When the filter is empty, all items are included. Otherwise, uses
    /// [`SkimMatcherV2`] for fuzzy matching against each item's
    /// [`display_label`](PickerItem::display_label). Preserves the consumer's
    /// original order.
    fn recompute_filtered(&mut self) {
        if self.filter.is_empty() {
            self.filtered_indices = (0..self.items.len()).collect();
        } else {
            let matcher = SkimMatcherV2::default();
            self.filtered_indices = self
                .items
                .iter()
                .enumerate()
                .filter_map(|(i, item)| {
                    matcher
                        .fuzzy_match(item.display_label(), &self.filter)
                        .map(|_| i)
                })
                .collect();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::text::Line;

    /// A minimal item type for testing. Intentionally does **not** derive `Clone`
    /// to verify that `SelectionState` works without it.
    #[derive(Debug)]
    struct TestItem {
        label: String,
    }

    impl TestItem {
        fn new(label: &str) -> Self {
            Self {
                label: label.to_owned(),
            }
        }
    }

    impl PickerItem for TestItem {
        fn display_label(&self) -> &str {
            &self.label
        }

        fn render_row(&self, _is_selected: bool) -> Line<'static> {
            Line::from(self.label.clone())
        }
    }

    /// Creates a list of test items from the given labels.
    fn make_items(labels: &[&str]) -> Vec<TestItem> {
        labels.iter().map(|&l| TestItem::new(l)).collect()
    }

    // =========================================================================
    // Ported from ProviderPickerState tests
    // =========================================================================

    // --- insert_char tests ---

    #[test]
    fn insert_char_appends_to_filter() {
        // Given a fresh selection state.
        let mut state = SelectionState::<TestItem>::new();

        // When inserting 'a' then 'b'.
        state.insert_char('a');
        state.insert_char('b');

        // Then the filter is "ab".
        assert_eq!(state.filter(), "ab");
    }

    #[test]
    fn insert_char_resets_selection() {
        // Given a selection state with selection at 3 and scroll_offset at 2.
        let mut state = SelectionState::<TestItem>::new();
        state.selection = 3;
        state.scroll_offset = 2;

        // When inserting a character.
        state.insert_char('x');

        // Then selection and scroll_offset are reset to 0.
        assert_eq!(state.selection(), 0);
        assert_eq!(state.scroll_offset(), 0);
    }

    #[test]
    fn insert_char_at_cursor_middle() {
        // Given a selection state with filter "abc" and cursor at 1.
        let mut state = SelectionState::<TestItem>::new();
        state.filter = "abc".to_owned();
        state.cursor_pos = 1;

        // When inserting 'x' at cursor.
        state.insert_char('x');

        // Then the filter is "axbc" and cursor advanced to 2.
        assert_eq!(state.filter(), "axbc");
        assert_eq!(state.cursor_pos(), 2);
    }

    #[test]
    fn insert_char_advances_cursor() {
        // Given a fresh selection state.
        let mut state = SelectionState::<TestItem>::new();

        // When inserting 'a'.
        state.insert_char('a');

        // Then cursor is at 1.
        assert_eq!(state.cursor_pos(), 1);
    }

    // --- backspace tests ---

    #[test]
    fn backspace_removes_before_cursor() {
        // Given a selection state with filter "ab" and cursor at end (position 2).
        let mut state = SelectionState::<TestItem>::new();
        state.filter = "ab".to_owned();
        state.cursor_pos = 2;

        // When pressing backspace.
        state.backspace();

        // Then the filter is "a" and cursor is at 1.
        assert_eq!(state.filter(), "a");
        assert_eq!(state.cursor_pos(), 1);
    }

    #[test]
    fn backspace_resets_selection() {
        // Given a selection state with filter "ab", cursor at end (2), selection at 3, and scroll_offset at 2.
        let mut state = SelectionState::<TestItem>::new();
        state.filter = "ab".to_owned();
        state.cursor_pos = 2;
        state.selection = 3;
        state.scroll_offset = 2;

        // When pressing backspace.
        state.backspace();

        // Then selection and scroll_offset are reset to 0.
        assert_eq!(state.selection(), 0);
        assert_eq!(state.scroll_offset(), 0);
    }

    #[test]
    fn backspace_at_cursor_middle() {
        // Given a selection state with filter "abc" and cursor at 2.
        let mut state = SelectionState::<TestItem>::new();
        state.filter = "abc".to_owned();
        state.cursor_pos = 2;

        // When pressing backspace.
        state.backspace();

        // Then the filter is "ac" and cursor is at 1.
        assert_eq!(state.filter(), "ac");
        assert_eq!(state.cursor_pos(), 1);
    }

    #[test]
    fn backspace_at_start_is_noop() {
        // Given a selection state with filter "abc" and cursor at 0.
        let mut state = SelectionState::<TestItem>::new();
        state.filter = "abc".to_owned();
        state.cursor_pos = 0;

        // When pressing backspace.
        state.backspace();

        // Then filter is unchanged and cursor is still 0.
        assert_eq!(state.filter(), "abc");
        assert_eq!(state.cursor_pos(), 0);
    }

    // --- cursor movement tests ---

    #[test]
    fn move_cursor_left_decrements() {
        // Given a selection state with cursor at 3.
        let mut state = SelectionState::<TestItem>::new();
        state.cursor_pos = 3;

        // When moving cursor left.
        state.move_cursor_left();

        // Then cursor is at 2.
        assert_eq!(state.cursor_pos(), 2);
    }

    #[test]
    fn move_cursor_left_clamps_at_zero() {
        // Given a selection state with cursor at 0.
        let mut state = SelectionState::<TestItem>::new();

        // When moving cursor left.
        state.move_cursor_left();

        // Then cursor stays at 0.
        assert_eq!(state.cursor_pos(), 0);
    }

    #[test]
    fn move_cursor_right_increments() {
        // Given a selection state with filter "abc" and cursor at 1.
        let mut state = SelectionState::<TestItem>::new();
        state.filter = "abc".to_owned();
        state.cursor_pos = 1;

        // When moving cursor right.
        state.move_cursor_right();

        // Then cursor is at 2.
        assert_eq!(state.cursor_pos(), 2);
    }

    #[test]
    fn move_cursor_right_clamps_at_end() {
        // Given a selection state with filter "abc" and cursor at 3 (end).
        let mut state = SelectionState::<TestItem>::new();
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
        // Given a selection state with 5 items and selection at 3.
        let mut state = SelectionState::with_items(make_items(&["a", "b", "c", "d", "e"]));
        state.selection = 3;

        // When moving up with max_visible=5.
        state.move_up(5);

        // Then selection is 2.
        assert_eq!(state.selection(), 2);
    }

    #[test]
    fn move_up_clamps_at_zero() {
        // Given a selection state with 5 items and selection at 0.
        let mut state = SelectionState::with_items(make_items(&["a", "b", "c", "d", "e"]));

        // When moving up.
        state.move_up(5);

        // Then selection stays at 0.
        assert_eq!(state.selection(), 0);
    }

    #[test]
    fn move_down_increments() {
        // Given a selection state with 5 items and selection at 1.
        let mut state = SelectionState::with_items(make_items(&["a", "b", "c", "d", "e"]));
        state.selection = 1;

        // When moving down with max_visible=5.
        state.move_down(5);

        // Then selection is 2.
        assert_eq!(state.selection(), 2);
    }

    #[test]
    fn move_down_clamps_at_max() {
        // Given a selection state with 5 items and selection at 4.
        let mut state = SelectionState::with_items(make_items(&["a", "b", "c", "d", "e"]));
        state.selection = 4;

        // When moving down.
        state.move_down(5);

        // Then selection stays at 4.
        assert_eq!(state.selection(), 4);
    }

    #[test]
    fn move_down_clamps_when_empty() {
        // Given a selection state with no items and selection at 0.
        let mut state = SelectionState::<TestItem>::new();

        // When moving down.
        state.move_down(5);

        // Then selection stays at 0.
        assert_eq!(state.selection(), 0);
    }

    // --- scroll offset tests ---

    #[test]
    fn move_up_adjusts_scroll_offset_when_selection_above_view() {
        // Given a selection state with 10 items, scroll_offset=2 and selection=2.
        let mut state =
            SelectionState::with_items((0..10).map(|i| TestItem::new(&i.to_string())).collect());
        state.selection = 2;
        state.scroll_offset = 2;

        // When moving up with max_visible=5.
        state.move_up(5);

        // Then selection is 1 and scroll_offset adjusts to 1.
        assert_eq!(state.selection(), 1);
        assert_eq!(state.scroll_offset(), 1);
    }

    #[test]
    fn move_down_adjusts_scroll_offset_when_selection_below_view() {
        // Given a selection state with 10 items, scroll_offset=0, selection=4.
        let mut state =
            SelectionState::with_items((0..10).map(|i| TestItem::new(&i.to_string())).collect());
        state.selection = 4;
        state.scroll_offset = 0;

        // When moving down with max_visible=5.
        state.move_down(5);

        // Then selection is 5 and scroll_offset adjusts to 1.
        assert_eq!(state.selection(), 5);
        assert_eq!(state.scroll_offset(), 1);
    }

    #[test]
    fn ensure_visible_selection_within_view() {
        // Given a selection state with scroll_offset=2 and selection=3.
        let mut state = SelectionState::<TestItem>::new();
        state.scroll_offset = 2;
        state.selection = 3;

        // When ensuring visible with max_visible=5.
        state.ensure_visible(5);

        // Then scroll_offset stays at 2 (3 is within [2, 7)).
        assert_eq!(state.scroll_offset(), 2);
    }

    #[test]
    fn ensure_visible_selection_above_view() {
        // Given a selection state with scroll_offset=3 and selection=1.
        let mut state = SelectionState::<TestItem>::new();
        state.scroll_offset = 3;
        state.selection = 1;

        // When ensuring visible with max_visible=5.
        state.ensure_visible(5);

        // Then scroll_offset adjusts up to 1.
        assert_eq!(state.scroll_offset(), 1);
    }

    #[test]
    fn ensure_visible_selection_below_view() {
        // Given a selection state with scroll_offset=0 and selection=7.
        let mut state = SelectionState::<TestItem>::new();
        state.scroll_offset = 0;
        state.selection = 7;

        // When ensuring visible with max_visible=5.
        state.ensure_visible(5);

        // Then scroll_offset adjusts down to 3.
        assert_eq!(state.scroll_offset(), 3);
    }

    // --- reset tests ---

    #[test]
    fn reset_clears_everything() {
        // Given a selection state with filter "ab", selection 3, cursor at 2, scroll_offset 1.
        let mut state = SelectionState::<TestItem>::new();
        state.filter = "ab".to_owned();
        state.selection = 3;
        state.cursor_pos = 2;
        state.scroll_offset = 1;

        // When resetting.
        state.reset();

        // Then filter is empty, selection is 0, cursor is 0, scroll_offset is 0.
        assert!(state.filter().is_empty());
        assert_eq!(state.selection(), 0);
        assert_eq!(state.cursor_pos(), 0);
        assert_eq!(state.scroll_offset(), 0);
    }

    // =========================================================================
    // New filtering tests
    // =========================================================================

    #[test]
    fn set_items_populates_filtered_list() {
        // Given a fresh selection state.
        let mut state = SelectionState::<TestItem>::new();

        // When setting 3 items.
        state.set_items(make_items(&["apple", "banana", "cherry"]));

        // Then all 3 items are in the filtered list.
        assert_eq!(state.filtered_count(), 3);
    }

    #[test]
    fn insert_char_filters_items_by_label() {
        // Given a selection state with items ["apple", "banana", "cherry"].
        let mut state = SelectionState::with_items(make_items(&["apple", "banana", "cherry"]));

        // When inserting 'a' to filter.
        state.insert_char('a');

        // Then filtered items contain "apple" and "banana" (fuzzy match) but not "cherry".
        assert_eq!(state.filtered_count(), 2);
        assert_eq!(state.filtered_item(0).unwrap().display_label(), "apple");
        assert_eq!(state.filtered_item(1).unwrap().display_label(), "banana");
    }

    #[test]
    fn backspace_re_expands_filtered_list() {
        // Given a selection state with items, filtered to "ap".
        let mut state = SelectionState::with_items(make_items(&["apple", "banana", "cherry"]));
        state.insert_char('a');
        state.insert_char('p');

        // When pressing backspace to remove 'p'.
        state.backspace();

        // Then more items appear (filter is now "a").
        assert_eq!(state.filtered_count(), 2);
        assert_eq!(state.filtered_item(0).unwrap().display_label(), "apple");
        assert_eq!(state.filtered_item(1).unwrap().display_label(), "banana");
    }

    #[test]
    fn filter_preserves_consumer_order() {
        // Given items in a specific order, all containing 'a'.
        let mut state = SelectionState::with_items(make_items(&["banana", "apple", "avocado"]));

        // When filtering with 'a'.
        state.insert_char('a');

        // Then filtered list preserves the consumer's original order.
        assert_eq!(state.filtered_count(), 3);
        assert_eq!(state.filtered_item(0).unwrap().display_label(), "banana");
        assert_eq!(state.filtered_item(1).unwrap().display_label(), "apple");
        assert_eq!(state.filtered_item(2).unwrap().display_label(), "avocado");
    }

    #[test]
    fn empty_filter_shows_all_items() {
        // Given a selection state with 3 items and no filter text.
        let state = SelectionState::with_items(make_items(&["apple", "banana", "cherry"]));

        // Then all items are visible.
        assert_eq!(state.filtered_count(), 3);
    }

    #[test]
    fn no_match_returns_empty_filtered() {
        // Given a selection state with items.
        let mut state = SelectionState::with_items(make_items(&["apple", "banana", "cherry"]));

        // When filtering to "zzz".
        state.insert_char('z');
        state.insert_char('z');
        state.insert_char('z');

        // Then filtered list is empty.
        assert_eq!(state.filtered_count(), 0);
    }

    #[test]
    fn selected_item_returns_none_when_no_match() {
        // Given a selection state with items filtered to no matches.
        let mut state = SelectionState::with_items(make_items(&["apple", "banana"]));
        state.insert_char('z');

        // Then selected_item returns None.
        assert!(state.selected_item().is_none());
    }

    #[test]
    fn selected_item_returns_first_match_initially() {
        // Given a selection state with items and no filter.
        let items = make_items(&["apple", "banana", "cherry"]);
        let state = SelectionState::with_items(items);

        // Then selected_item returns the first item.
        let selected = state.selected_item().expect("should have a selected item");
        assert_eq!(selected.display_label(), "apple");
    }

    #[test]
    fn filtered_item_returns_by_filtered_index() {
        // Given items ["a", "b", "c"], filtered to match "b" and "c".
        let mut state = SelectionState::with_items(make_items(&["alpha", "bravo", "charlie"]));
        state.insert_char('r');

        // Then filtered_item(0) is "bravo" and filtered_item(1) is "charlie".
        assert_eq!(state.filtered_count(), 2);
        assert_eq!(state.filtered_item(0).unwrap().display_label(), "bravo");
        assert_eq!(state.filtered_item(1).unwrap().display_label(), "charlie");
    }

    #[test]
    fn fuzzy_match_matches_partial() {
        // Given an item "hello world".
        let mut state = SelectionState::with_items(make_items(&["hello world"]));

        // When filtering with "hlo".
        state.insert_char('h');
        state.insert_char('l');
        state.insert_char('o');

        // Then the item matches.
        assert_eq!(state.filtered_count(), 1);
    }

    #[test]
    fn fuzzy_match_is_case_insensitive() {
        // Given an item "Hello" and filter "hello".
        let mut state = SelectionState::with_items(make_items(&["Hello"]));

        // When filtering with "hello".
        for ch in "hello".chars() {
            state.insert_char(ch);
        }

        // Then the item matches (case-insensitive).
        assert_eq!(state.filtered_count(), 1);
    }

    #[test]
    fn set_items_does_not_reset_filter() {
        // Given a selection state with items and a filter.
        let mut state = SelectionState::with_items(make_items(&["apple", "banana"]));
        state.insert_char('a');

        // When setting new items.
        state.set_items(make_items(&["apple", "banana", "cherry"]));

        // Then filter text persists and filtered list updates.
        assert_eq!(state.filter(), "a");
        assert_eq!(state.filtered_count(), 2);
    }

    #[test]
    fn reset_clears_filter_but_keeps_items() {
        // Given a selection state with items and a filter.
        let mut state = SelectionState::with_items(make_items(&["apple", "banana", "cherry"]));
        state.insert_char('a');

        // When resetting.
        state.reset();

        // Then filter is empty but items are still present.
        assert!(state.filter().is_empty());
        assert_eq!(state.items().len(), 3);
        assert_eq!(state.filtered_count(), 3);
    }

    #[test]
    fn no_clone_needed() {
        // Compile-time proof: TestItem does not derive Clone,
        // yet SelectionState<TestItem> works fine.
        let mut state = SelectionState::<TestItem>::new();
        state.set_items(vec![TestItem::new("test")]);
        assert_eq!(state.filtered_count(), 1);
    }
}
