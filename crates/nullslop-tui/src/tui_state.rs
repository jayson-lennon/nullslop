//! Ephemeral UI state for the TUI layer.
//!
//! This state is main-thread only and not shared across threads.
//! It contains rendering concerns like the input buffer and scroll offset.

use unicode_segmentation::UnicodeSegmentation;

/// Mutable state for the TUI layer.
///
/// Owned by the application loop and passed to command handlers
/// and render functions.
#[derive(Debug, Default)]
pub struct TuiState {
    /// The current text in the input box.
    pub input_buffer: String,
    /// The scroll offset for the chat log (in lines from bottom).
    pub scroll_offset: u16,
}

impl TuiState {
    /// Creates a new empty TUI state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a character to the input buffer.
    pub fn push_char(&mut self, c: char) {
        self.input_buffer.push(c);
    }

    /// Removes the last grapheme cluster from the input buffer.
    ///
    /// Uses `unicode_segmentation` to handle multi-byte characters correctly.
    pub fn pop_grapheme(&mut self) {
        if let Some((idx, _)) = self.input_buffer.grapheme_indices(true).next_back() {
            self.input_buffer.truncate(idx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_state_has_empty_buffer() {
        // Given a new TuiState.
        let state = TuiState::new();

        // Then input_buffer is empty.
        assert!(state.input_buffer.is_empty());
    }

    #[test]
    fn push_char_appends() {
        // Given a TuiState.
        let mut state = TuiState::new();

        // When pushing 'a' then 'b'.
        state.push_char('a');
        state.push_char('b');

        // Then buffer is "ab".
        assert_eq!(state.input_buffer, "ab");
    }

    #[test]
    fn pop_grapheme_removes_last() {
        // Given buffer "abc".
        let mut state = TuiState::new();
        state.push_char('a');
        state.push_char('b');
        state.push_char('c');

        // When popping.
        state.pop_grapheme();

        // Then buffer is "ab".
        assert_eq!(state.input_buffer, "ab");
    }

    #[test]
    fn pop_grapheme_handles_unicode() {
        // Given buffer "é" (multi-byte grapheme).
        let mut state = TuiState::new();
        state.input_buffer = "é".to_string();

        // When popping.
        state.pop_grapheme();

        // Then buffer is "".
        assert_eq!(state.input_buffer, "");
    }

    #[test]
    fn pop_grapheme_empty_is_noop() {
        // Given empty buffer.
        let mut state = TuiState::new();

        // When popping.
        state.pop_grapheme();

        // Then buffer is still empty.
        assert!(state.input_buffer.is_empty());
    }
}
