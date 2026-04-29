//! State for the chat input box.
//!
//! [`ChatInputBoxState`] encapsulates the input buffer and related operations,
//! keeping input-box concerns together in one place.

use unicode_segmentation::UnicodeSegmentation;

/// Encapsulated state for the chat input box.
///
/// Holds the input buffer and provides methods for manipulating it.
/// Colocated with the chat input box component via re-export from
/// `nullslop_component::chat_input_box`.
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
        let state = ChatInputBoxState::new();
        assert!(state.input_buffer.is_empty());
    }

    #[test]
    fn pop_grapheme_removes_last() {
        let mut state = ChatInputBoxState::new();
        state.input_buffer = "abc".to_string();
        state.pop_grapheme();
        assert_eq!(state.input_buffer, "ab");
    }

    #[test]
    fn pop_grapheme_handles_unicode() {
        let mut state = ChatInputBoxState::new();
        state.input_buffer = "é".to_string();
        state.pop_grapheme();
        assert_eq!(state.input_buffer, "");
    }

    #[test]
    fn pop_grapheme_empty_is_noop() {
        let mut state = ChatInputBoxState::new();
        state.pop_grapheme();
        assert!(state.input_buffer.is_empty());
    }
}
