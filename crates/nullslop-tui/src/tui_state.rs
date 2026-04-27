//! Ephemeral UI state for the TUI layer.
//!
//! This state is main-thread only and not shared across threads.
//! It contains rendering concerns like scroll offset.
//!
//! Note: `input_buffer` lives in [`AppData`](nullslop_protocol::AppData),
//! not here. The plugin system operates on `AppData` via the bus.

/// Mutable state for the TUI layer.
///
/// Owned by the application loop and passed to render functions.
/// Domain-level state (input buffer, chat history, mode) lives in
/// [`AppData`](nullslop_protocol::AppData).
#[derive(Debug, Default)]
pub struct TuiState {
    /// The scroll offset for the chat log (in lines from bottom).
    pub scroll_offset: u16,
}

impl TuiState {
    /// Creates a new empty TUI state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_state_has_zero_scroll() {
        // Given a new TuiState.
        let state = TuiState::new();

        // Then scroll_offset is zero.
        assert_eq!(state.scroll_offset, 0);
    }
}
