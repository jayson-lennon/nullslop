//! State for the chat input box.
//!
//! [`ChatInputBoxState`] encapsulates the input buffer and related operations,
//! keeping input-box concerns together in one place.

use unicode_segmentation::UnicodeSegmentation;

/// Encapsulated state for the chat input box.
///
/// Holds the input buffer and provides methods for manipulating it.
/// Colocated with the chat input box plugin via re-export from
/// `nullslop_plugin::chat_input_box`.
#[derive(Debug)]
pub struct ChatInputBoxState {
    /// The current text in the input box.
    pub input_buffer: String,
}

impl ChatInputBoxState {
    /// Create a new `ChatInputBoxState` with an empty buffer.
    #[must_use]
    pub fn new() -> Self {
        Self {
            input_buffer: String::new(),
        }
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
        // Given a new ChatInputBoxState.
        let state = ChatInputBoxState::new();

        // Then input_buffer is empty.
        assert!(state.input_buffer.is_empty());
    }

    #[test]
    fn pop_grapheme_removes_last() {
        // Given state with "abc" in the buffer.
        let mut state = ChatInputBoxState::new();
        state.input_buffer = "abc".to_string();

        // When popping a grapheme.
        state.pop_grapheme();

        // Then buffer is "ab".
        assert_eq!(state.input_buffer, "ab");
    }

    #[test]
    fn pop_grapheme_handles_unicode() {
        // Given state with "é" in the buffer.
        let mut state = ChatInputBoxState::new();
        state.input_buffer = "é".to_string();

        // When popping a grapheme.
        state.pop_grapheme();

        // Then buffer is empty.
        assert_eq!(state.input_buffer, "");
    }

    #[test]
    fn pop_grapheme_empty_is_noop() {
        // Given state with empty buffer.
        let mut state = ChatInputBoxState::new();

        // When popping a grapheme.
        state.pop_grapheme();

        // Then buffer is still empty.
        assert!(state.input_buffer.is_empty());
    }
}
