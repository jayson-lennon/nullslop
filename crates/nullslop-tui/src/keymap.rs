//! Keymap configuration and initialization.
//!
//! Defines the key categories and builds the keymap with all scope bindings
//! using the `ratatui-which-key` crate.

use derive_more::Display;
use ratatui_which_key::Keymap;

use crate::command::TuiCommand;
use crate::scope::Scope;

/// Categories for keybinding grouping in the which-key popup.
#[derive(Display, Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCategory {
    /// General-purpose actions like quit and help.
    General,
    /// Input-related actions like submit and edit.
    Input,
}

/// Builds and returns the full keymap with all scope bindings.
#[must_use]
pub fn init() -> Keymap<crossterm::event::KeyEvent, Scope, TuiCommand, KeyCategory> {
    let mut keymap = Keymap::new();

    keymap
        // Normal scope: navigation and commands
        .scope(Scope::Normal, |b| {
            b.bind("<enter>", TuiCommand::EnterInput, KeyCategory::General)
                .bind("<esc>", TuiCommand::Quit, KeyCategory::General)
                .bind("?", TuiCommand::ToggleWhichKey, KeyCategory::General)
                .bind("<c-e>", TuiCommand::EditInput, KeyCategory::Input);
        })
        // Input scope: typing into the input buffer
        .scope(Scope::Input, |b| {
            b.bind("<enter>", TuiCommand::SubmitChat, KeyCategory::Input)
                .bind("<esc>", TuiCommand::BackToNormal, KeyCategory::General)
                .bind("<c-e>", TuiCommand::EditInput, KeyCategory::Input)
                .bind(
                    "<backspace>",
                    TuiCommand::DeleteGrapheme,
                    KeyCategory::Input,
                )
                .catch_all(|key: crossterm::event::KeyEvent| {
                    use crossterm::event::{KeyCode, KeyEventKind};
                    // Only handle Press events (crossterm fires Release/Repeat too)
                    if key.kind != KeyEventKind::Press {
                        return None;
                    }
                    if let KeyCode::Char(c) = key.code {
                        Some(TuiCommand::InsertChar(c))
                    } else {
                        None
                    }
                });
        });

    keymap
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::*;

    #[test]
    fn keymap_init_normal_scope_has_enter_binding() {
        // Given an initialized keymap.
        let keymap = init();
        let mut state = ratatui_which_key::WhichKeyState::new(keymap, Scope::Normal);

        // When navigating Normal scope with Enter key.
        let result = state.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        // Then returns TuiCommand::EnterInput.
        assert_eq!(result, Some(TuiCommand::EnterInput));
    }

    #[test]
    fn keymap_init_input_scope_has_submit_binding() {
        // Given an initialized keymap.
        let keymap = init();
        let mut state = ratatui_which_key::WhichKeyState::new(keymap, Scope::Input);

        // When navigating Input scope with Enter key.
        let result = state.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        // Then returns TuiCommand::SubmitChat.
        assert_eq!(result, Some(TuiCommand::SubmitChat));
    }

    #[test]
    fn keymap_init_input_scope_catch_all_handles_char() {
        // Given an initialized keymap in Input scope.
        let keymap = init();
        let mut state = ratatui_which_key::WhichKeyState::new(keymap, Scope::Input);

        // When pressing 'a' (no explicit binding).
        let result = state.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));

        // Then catch_all returns TuiCommand::InsertChar('a').
        assert_eq!(result, Some(TuiCommand::InsertChar('a')));
    }

    #[test]
    fn keymap_init_normal_scope_esc_quits() {
        // Given an initialized keymap.
        let keymap = init();
        let mut state = ratatui_which_key::WhichKeyState::new(keymap, Scope::Normal);

        // When navigating Normal scope with Esc.
        let result = state.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

        // Then returns TuiCommand::Quit.
        assert_eq!(result, Some(TuiCommand::Quit));
    }
}
