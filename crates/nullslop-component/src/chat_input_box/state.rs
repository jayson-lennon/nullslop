//! Input buffer for the chat input box.
//!
//! Holds the user's in-progress message — the text they have typed but not yet sent.
//! Tracks cursor position as a grapheme-cluster index so that insert and delete
//! operations work correctly at any position in the buffer.

use unicode_segmentation::UnicodeSegmentation;

/// The user's in-progress message being composed in the input box.
///
/// Both the text buffer and cursor position are private. All mutation goes through
/// semantic methods that keep the cursor in sync with the buffer content.
#[derive(Debug)]
pub struct ChatInputBoxState {
    /// The text the user has typed so far.
    input_buffer: String,
    /// Cursor position as a grapheme-cluster index (0 = before first grapheme).
    cursor_pos: usize,
}

impl ChatInputBoxState {
    /// Create a new state with no text entered and cursor at position 0.
    #[must_use]
    pub fn new() -> Self {
        Self {
            input_buffer: String::new(),
            cursor_pos: 0,
        }
    }

    /// Returns a reference to the current input text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.input_buffer
    }

    /// Returns whether the input buffer is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.input_buffer.is_empty()
    }

    /// Returns the current cursor position as a grapheme index.
    #[must_use]
    pub fn cursor_pos(&self) -> usize {
        self.cursor_pos
    }

    /// Returns the total number of grapheme clusters in the buffer.
    #[must_use]
    pub fn grapheme_count(&self) -> usize {
        self.input_buffer.graphemes(true).count()
    }

    /// Insert a character at the current cursor position and advance the cursor by 1.
    pub fn insert_grapheme_at_cursor(&mut self, ch: char) {
        let byte_offset = self
            .input_buffer
            .grapheme_indices(true)
            .nth(self.cursor_pos)
            .map_or(self.input_buffer.len(), |(i, _)| i);
        self.input_buffer.insert(byte_offset, ch);
        self.cursor_pos += 1;
    }

    /// Delete the grapheme immediately before the cursor and move the cursor back by 1.
    ///
    /// No-op when the cursor is at position 0.
    pub fn delete_grapheme_before_cursor(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }
        let graphemes: Vec<(usize, &str)> = self.input_buffer.grapheme_indices(true).collect();
        let delete_idx = self.cursor_pos - 1;
        let (start, g) = graphemes[delete_idx];
        let end = start + g.len();
        self.input_buffer.drain(start..end);
        self.cursor_pos -= 1;
    }

    /// Clear the buffer and reset the cursor to position 0.
    pub fn reset(&mut self) {
        self.input_buffer.clear();
        self.cursor_pos = 0;
    }

    /// Replace the entire buffer content and position cursor at the end.
    ///
    /// Used when loading content from an external editor.
    pub fn replace_all(&mut self, content: String) {
        self.input_buffer = content;
        self.cursor_pos = self.input_buffer.graphemes(true).count();
    }

    /// Move the cursor one grapheme to the left.
    ///
    /// No-op when the cursor is already at position 0.
    pub fn move_cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    /// Move the cursor one grapheme to the right.
    ///
    /// No-op when the cursor is already at the end of the buffer.
    pub fn move_cursor_right(&mut self) {
        if self.cursor_pos < self.grapheme_count() {
            self.cursor_pos += 1;
        }
    }

    /// Move the cursor to the beginning of the buffer.
    pub fn move_cursor_to_start(&mut self) {
        self.cursor_pos = 0;
    }

    /// Move the cursor to the end of the buffer.
    pub fn move_cursor_to_end(&mut self) {
        self.cursor_pos = self.grapheme_count();
    }

    /// Delete the grapheme at the cursor position (forward delete).
    ///
    /// No-op when the cursor is at the end of the buffer.
    pub fn delete_grapheme_after_cursor(&mut self) {
        let count = self.grapheme_count();
        if self.cursor_pos >= count {
            return;
        }
        let graphemes: Vec<(usize, &str)> = self.input_buffer.grapheme_indices(true).collect();
        let (start, g) = graphemes[self.cursor_pos];
        let end = start + g.len();
        self.input_buffer.drain(start..end);
    }

    /// Move the cursor one word to the left.
    ///
    /// A word boundary is a transition from whitespace to non-whitespace.
    /// Scans left from the current cursor position, skips any whitespace,
    /// then finds the start of the preceding word.
    /// No-op when the cursor is at position 0.
    pub fn move_cursor_word_left(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }
        let graphemes: Vec<&str> = self.input_buffer.graphemes(true).collect();
        let mut pos = self.cursor_pos;

        // Skip whitespace moving left.
        while pos > 0 && graphemes[pos - 1].trim().is_empty() {
            pos -= 1;
        }
        // Skip non-whitespace moving left (the word itself).
        while pos > 0 && !graphemes[pos - 1].trim().is_empty() {
            pos -= 1;
        }
        self.cursor_pos = pos;
    }

    /// Move the cursor one word to the right.
    ///
    /// A word boundary is a transition from non-whitespace to whitespace.
    /// Scans right from the current cursor position, skips any non-whitespace,
    /// then skips any whitespace to land at the start of the next word.
    /// No-op when the cursor is at the end of the buffer.
    pub fn move_cursor_word_right(&mut self) {
        let count = self.grapheme_count();
        if self.cursor_pos >= count {
            return;
        }
        let graphemes: Vec<&str> = self.input_buffer.graphemes(true).collect();
        let mut pos = self.cursor_pos;

        // Skip non-whitespace moving right (the current word).
        while pos < count && !graphemes[pos].trim().is_empty() {
            pos += 1;
        }
        // Skip whitespace moving right.
        while pos < count && graphemes[pos].trim().is_empty() {
            pos += 1;
        }
        self.cursor_pos = pos;
    }
}

impl Default for ChatInputBoxState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_state_has_empty_buffer() {
        let state = ChatInputBoxState::new();
        assert!(state.is_empty());
        assert_eq!(state.text(), "");
    }

    #[test]
    fn new_state_has_cursor_at_zero() {
        let state = ChatInputBoxState::new();
        assert_eq!(state.cursor_pos(), 0);
    }

    #[test]
    fn text_returns_buffer_content() {
        let mut state = ChatInputBoxState::new();
        state.insert_grapheme_at_cursor('h');
        state.insert_grapheme_at_cursor('i');
        assert_eq!(state.text(), "hi");
    }

    #[test]
    fn is_empty_reflects_buffer_state() {
        let mut state = ChatInputBoxState::new();
        assert!(state.is_empty());
        state.insert_grapheme_at_cursor('a');
        assert!(!state.is_empty());
    }

    #[test]
    fn grapheme_count_returns_cluster_count() {
        let mut state = ChatInputBoxState::new();
        for ch in "éNoël".chars() {
            state.insert_grapheme_at_cursor(ch);
        }
        assert_eq!(state.grapheme_count(), 5);
    }

    #[test]
    fn insert_grapheme_at_cursor_appends_to_empty_buffer() {
        let mut state = ChatInputBoxState::new();
        state.insert_grapheme_at_cursor('X');
        assert_eq!(state.text(), "X");
        assert_eq!(state.cursor_pos(), 1);
    }

    #[test]
    fn insert_grapheme_at_cursor_sequential() {
        let mut state = ChatInputBoxState::new();
        state.insert_grapheme_at_cursor('a');
        state.insert_grapheme_at_cursor('b');
        state.insert_grapheme_at_cursor('c');
        assert_eq!(state.text(), "abc");
        assert_eq!(state.cursor_pos(), 3);
    }

    #[test]
    fn cursor_pos_advances_with_each_insert() {
        let mut state = ChatInputBoxState::new();
        assert_eq!(state.cursor_pos(), 0);
        state.insert_grapheme_at_cursor('a');
        assert_eq!(state.cursor_pos(), 1);
        state.insert_grapheme_at_cursor('b');
        assert_eq!(state.cursor_pos(), 2);
    }

    #[test]
    fn delete_grapheme_before_cursor_removes_last() {
        let mut state = ChatInputBoxState::new();
        state.insert_grapheme_at_cursor('a');
        state.insert_grapheme_at_cursor('b');
        state.insert_grapheme_at_cursor('c');
        state.delete_grapheme_before_cursor();
        assert_eq!(state.text(), "ab");
        assert_eq!(state.cursor_pos(), 2);
    }

    #[test]
    fn delete_grapheme_before_cursor_handles_unicode() {
        let mut state = ChatInputBoxState::new();
        state.insert_grapheme_at_cursor('é');
        state.delete_grapheme_before_cursor();
        assert_eq!(state.text(), "");
        assert_eq!(state.cursor_pos(), 0);
    }

    #[test]
    fn delete_grapheme_before_cursor_empty_is_noop() {
        let mut state = ChatInputBoxState::new();
        state.delete_grapheme_before_cursor();
        assert!(state.is_empty());
        assert_eq!(state.cursor_pos(), 0);
    }

    #[test]
    fn replace_all_sets_text_and_cursor_at_end() {
        let mut state = ChatInputBoxState::new();
        state.replace_all("hello".to_string());
        assert_eq!(state.text(), "hello");
        assert_eq!(state.cursor_pos(), 5);
    }

    #[test]
    fn reset_clears_buffer_and_cursor() {
        let mut state = ChatInputBoxState::new();
        state.insert_grapheme_at_cursor('a');
        state.insert_grapheme_at_cursor('b');
        state.insert_grapheme_at_cursor('c');
        state.reset();
        assert!(state.is_empty());
        assert_eq!(state.cursor_pos(), 0);
    }

    // --- Phase 2: cursor movement tests ---

    #[test]
    fn move_cursor_left_decrements_position() {
        // Given "abc" with cursor at end (3).
        let mut state = ChatInputBoxState::new();
        for ch in "abc".chars() {
            state.insert_grapheme_at_cursor(ch);
        }

        // When moving left.
        state.move_cursor_left();

        // Then cursor is at 2 and text unchanged.
        assert_eq!(state.cursor_pos(), 2);
        assert_eq!(state.text(), "abc");
    }

    #[test]
    fn move_cursor_left_clamps_at_zero() {
        // Given empty state with cursor at 0.
        let mut state = ChatInputBoxState::new();

        // When moving left.
        state.move_cursor_left();

        // Then cursor is still at 0.
        assert_eq!(state.cursor_pos(), 0);
    }

    #[test]
    fn move_cursor_right_increments_position() {
        // Given "abc" with cursor moved to start.
        let mut state = ChatInputBoxState::new();
        for ch in "abc".chars() {
            state.insert_grapheme_at_cursor(ch);
        }
        state.move_cursor_to_start();

        // When moving right.
        state.move_cursor_right();

        // Then cursor is at 1.
        assert_eq!(state.cursor_pos(), 1);
    }

    #[test]
    fn move_cursor_right_clamps_at_end() {
        // Given "a" with cursor at end (1).
        let mut state = ChatInputBoxState::new();
        state.insert_grapheme_at_cursor('a');

        // When moving right.
        state.move_cursor_right();

        // Then cursor is still at 1.
        assert_eq!(state.cursor_pos(), 1);
    }

    #[test]
    fn move_cursor_to_start_sets_zero() {
        // Given "abc" with cursor at end (3).
        let mut state = ChatInputBoxState::new();
        for ch in "abc".chars() {
            state.insert_grapheme_at_cursor(ch);
        }

        // When moving to start.
        state.move_cursor_to_start();

        // Then cursor is at 0.
        assert_eq!(state.cursor_pos(), 0);
    }

    #[test]
    fn move_cursor_to_end_sets_count() {
        // Given "abc" with cursor moved to start.
        let mut state = ChatInputBoxState::new();
        for ch in "abc".chars() {
            state.insert_grapheme_at_cursor(ch);
        }
        state.move_cursor_to_start();

        // When moving to end.
        state.move_cursor_to_end();

        // Then cursor is at 3.
        assert_eq!(state.cursor_pos(), 3);
    }

    #[test]
    fn insert_at_mid_buffer() {
        // Given "abc" with cursor moved left to position 2.
        let mut state = ChatInputBoxState::new();
        for ch in "abc".chars() {
            state.insert_grapheme_at_cursor(ch);
        }
        state.move_cursor_left(); // cursor at 2

        // When inserting 'X'.
        state.insert_grapheme_at_cursor('X');

        // Then text is "abXc" and cursor at 3.
        assert_eq!(state.text(), "abXc");
        assert_eq!(state.cursor_pos(), 3);
    }

    #[test]
    fn delete_before_cursor_at_mid_buffer() {
        // Given "abc" with cursor moved left to position 2.
        let mut state = ChatInputBoxState::new();
        for ch in "abc".chars() {
            state.insert_grapheme_at_cursor(ch);
        }
        state.move_cursor_left(); // cursor at 2

        // When deleting before cursor.
        state.delete_grapheme_before_cursor();

        // Then text is "ac" and cursor at 1.
        assert_eq!(state.text(), "ac");
        assert_eq!(state.cursor_pos(), 1);
    }

    #[test]
    fn delete_after_cursor_at_mid_buffer() {
        // Given "abc" with cursor moved left to position 2.
        let mut state = ChatInputBoxState::new();
        for ch in "abc".chars() {
            state.insert_grapheme_at_cursor(ch);
        }
        state.move_cursor_left(); // cursor at 2

        // When deleting after cursor (forward delete).
        state.delete_grapheme_after_cursor();

        // Then text is "ab" and cursor still at 2.
        assert_eq!(state.text(), "ab");
        assert_eq!(state.cursor_pos(), 2);
    }

    #[test]
    fn delete_after_cursor_at_end_is_noop() {
        // Given "a" with cursor at end (1).
        let mut state = ChatInputBoxState::new();
        state.insert_grapheme_at_cursor('a');

        // When deleting after cursor.
        state.delete_grapheme_after_cursor();

        // Then text is still "a" and cursor still at 1.
        assert_eq!(state.text(), "a");
        assert_eq!(state.cursor_pos(), 1);
    }

    #[test]
    fn delete_after_cursor_at_start() {
        // Given "abc" with cursor at start (0).
        let mut state = ChatInputBoxState::new();
        for ch in "abc".chars() {
            state.insert_grapheme_at_cursor(ch);
        }
        state.move_cursor_to_start(); // cursor at 0

        // When deleting after cursor.
        state.delete_grapheme_after_cursor();

        // Then text is "bc" and cursor still at 0.
        assert_eq!(state.text(), "bc");
        assert_eq!(state.cursor_pos(), 0);
    }

    #[test]
    fn move_cursor_word_left_skips_word() {
        // Given "hello world" with cursor at end (11).
        let mut state = ChatInputBoxState::new();
        for ch in "hello world".chars() {
            state.insert_grapheme_at_cursor(ch);
        }

        // When moving word left.
        state.move_cursor_word_left();

        // Then cursor is at 6 (start of "world").
        assert_eq!(state.cursor_pos(), 6);
    }

    #[test]
    fn move_cursor_word_left_skips_leading_whitespace() {
        // Given "a  b" with cursor at end (4).
        let mut state = ChatInputBoxState::new();
        for ch in "a  b".chars() {
            state.insert_grapheme_at_cursor(ch);
        }

        // When moving word left twice.
        state.move_cursor_word_left();
        assert_eq!(state.cursor_pos(), 3);
        state.move_cursor_word_left();

        // Then cursor is at 0.
        assert_eq!(state.cursor_pos(), 0);
    }

    #[test]
    fn move_cursor_word_left_at_start_is_noop() {
        // Given empty state with cursor at 0.
        let mut state = ChatInputBoxState::new();

        // When moving word left.
        state.move_cursor_word_left();

        // Then cursor is still at 0.
        assert_eq!(state.cursor_pos(), 0);
    }

    #[test]
    fn move_cursor_word_right_skips_word() {
        // Given "hello world" with cursor at start (0).
        let mut state = ChatInputBoxState::new();
        for ch in "hello world".chars() {
            state.insert_grapheme_at_cursor(ch);
        }
        state.move_cursor_to_start();

        // When moving word right.
        state.move_cursor_word_right();

        // Then cursor is at 6 (start of "world").
        assert_eq!(state.cursor_pos(), 6);
    }

    #[test]
    fn move_cursor_word_right_skips_trailing_whitespace() {
        // Given "a  b" with cursor at start (0).
        let mut state = ChatInputBoxState::new();
        for ch in "a  b".chars() {
            state.insert_grapheme_at_cursor(ch);
        }
        state.move_cursor_to_start();

        // When moving word right twice.
        state.move_cursor_word_right();
        assert_eq!(state.cursor_pos(), 3);
        state.move_cursor_word_right();

        // Then cursor is at 4 (end).
        assert_eq!(state.cursor_pos(), 4);
    }

    #[test]
    fn move_cursor_word_right_at_end_is_noop() {
        // Given "a" with cursor at end (1).
        let mut state = ChatInputBoxState::new();
        state.insert_grapheme_at_cursor('a');

        // When moving word right.
        state.move_cursor_word_right();

        // Then cursor is still at 1.
        assert_eq!(state.cursor_pos(), 1);
    }

    #[test]
    fn move_cursor_left_right_with_unicode() {
        // Given "écafé" with cursor at end (5).
        let mut state = ChatInputBoxState::new();
        for ch in "écafé".chars() {
            state.insert_grapheme_at_cursor(ch);
        }

        // When moving left twice then right.
        state.move_cursor_left();
        state.move_cursor_left();
        assert_eq!(state.cursor_pos(), 3);
        state.move_cursor_right();

        // Then cursor is at 4.
        assert_eq!(state.cursor_pos(), 4);
    }

    #[test]
    fn word_boundaries_with_unicode() {
        // Given "café au lait" with cursor at start (0).
        let mut state = ChatInputBoxState::new();
        for ch in "café au lait".chars() {
            state.insert_grapheme_at_cursor(ch);
        }
        state.move_cursor_to_start();

        // When moving word right.
        state.move_cursor_word_right();

        // Then cursor is at 5 (after "café ").
        assert_eq!(state.cursor_pos(), 5);

        // When moving word right again.
        state.move_cursor_word_right();

        // Then cursor is at 8 (after "au ").
        assert_eq!(state.cursor_pos(), 8);
    }
}
