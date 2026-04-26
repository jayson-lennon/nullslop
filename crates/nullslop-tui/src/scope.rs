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
    fn scopes_are_equal_to_themselves() {
        // Given each scope variant.
        // Then it equals itself.
        assert_eq!(Scope::Normal, Scope::Normal);
        assert_eq!(Scope::Input, Scope::Input);
    }
}
