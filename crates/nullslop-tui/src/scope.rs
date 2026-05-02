//! Keymap scopes for context-sensitive key handling.
//!
//! The scope determines which set of keybindings is active.

/// The current keymap context.
///
/// Controls which keybindings are active. Set via
/// [`ratatui_which_key::WhichKeyState::set_scope`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Scope {
    /// Normal mode — navigation and commands.
    Normal,
    /// Picker mode — filtering and selecting a provider.
    Picker,
    /// Input mode — typing into the input buffer.
    Input,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_is_less_than_input() {
        // Given the two scopes.
        // When comparing.
        // Then Normal < Input.
        assert!(Scope::Normal < Scope::Input);
    }

    #[test]
    fn picker_is_between_normal_and_input() {
        // Given the three scopes.
        // When comparing.
        // Then Normal < Picker < Input (derived from declaration order).
        assert!(Scope::Normal < Scope::Picker);
        assert!(Scope::Picker < Scope::Input);
    }
}
