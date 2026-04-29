//! Input buffer for the chat input box.
//!
//! Holds the user's in-progress message — the text they have typed but not yet sent.

use unicode_segmentation::UnicodeSegmentation;

/// The user's in-progress message being composed in the input box.
#[derive(Debug)]
pub struct ChatInputBoxState {
    /// The text the user has typed so far.
    pub input_buffer: String,
}

impl ChatInputBoxState {
    /// Create a new state with no text entered.
    #[must_use]
    pub fn new() -> Self {
        Self {
            input_buffer: String::new(),
        }
    }

    /// Delete the last character the user typed, handling multi-byte Unicode correctly.
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
