//! Keymap configuration and initialization.
//!
//! Defines the key categories and builds the keymap with all scope bindings
//! using the `ratatui-which-key` crate. Binds keys to [`Command`](nullslop_protocol::Command)
//! variants. Parameterized on [`nullslop_core::KeyEvent`] so the keymap works
//! in both TUI and headless modes.

use derive_more::Display;
use nullslop_core::{KeyEvent, Key};
use nullslop_protocol::command::{AppSetMode, ChatBoxInsertChar, ChatBoxSubmitMessage};
use nullslop_protocol::{Command, Mode};
use ratatui_which_key::Keymap;

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
pub fn init() -> Keymap<KeyEvent, Scope, Command, KeyCategory> {
    let mut keymap = Keymap::new();

    keymap
        // Normal scope: navigation and commands
        .scope(Scope::Normal, |b| {
            b.bind(
                "<enter>",
                Command::AppSetMode {
                    payload: AppSetMode { mode: Mode::Input },
                },
                KeyCategory::General,
            )
            .bind("<esc>", Command::AppQuit, KeyCategory::General)
            .bind("?", Command::AppToggleWhichKey, KeyCategory::General)
            .bind("<c-e>", Command::AppEditInput, KeyCategory::Input);
        })
        // Input scope: typing into the input buffer
        .scope(Scope::Input, |b| {
            b.bind(
                "<enter>",
                Command::ChatBoxSubmitMessage {
                    payload: ChatBoxSubmitMessage {
                        text: String::new(),
                    },
                },
                KeyCategory::Input,
            )
            .bind(
                "<esc>",
                Command::AppSetMode {
                    payload: AppSetMode { mode: Mode::Normal },
                },
                KeyCategory::General,
            )
            .bind("<c-e>", Command::AppEditInput, KeyCategory::Input)
            .bind(
                "<backspace>",
                Command::ChatBoxDeleteGrapheme,
                KeyCategory::Input,
            )
            .catch_all(|key: KeyEvent| {
                if let Key::Char(c) = key.key {
                    Some(Command::ChatBoxInsertChar {
                        payload: ChatBoxInsertChar { ch: c },
                    })
                } else {
                    None
                }
            });
        });

    keymap
}

#[cfg(test)]
mod tests {
    use nullslop_core::{Key, KeyEvent, Modifiers};

    use super::*;

    fn key_event(key: Key) -> KeyEvent {
        KeyEvent {
            key,
            modifiers: Modifiers::none(),
        }
    }

    #[test]
    fn keymap_init_normal_scope_has_enter_binding() {
        // Given an initialized keymap.
        let keymap = init();
        let mut state = ratatui_which_key::WhichKeyState::new(keymap, Scope::Normal);

        // When navigating Normal scope with Enter key.
        let result = state.handle_key(key_event(Key::Enter));

        // Then returns AppSetMode with Input mode.
        assert!(matches!(
            result,
            Some(Command::AppSetMode {
                payload: AppSetMode { mode: Mode::Input }
            })
        ));
    }

    #[test]
    fn keymap_init_input_scope_has_submit_binding() {
        // Given an initialized keymap.
        let keymap = init();
        let mut state = ratatui_which_key::WhichKeyState::new(keymap, Scope::Input);

        // When navigating Input scope with Enter key.
        let result = state.handle_key(key_event(Key::Enter));

        // Then returns ChatBoxSubmitMessage.
        assert!(matches!(result, Some(Command::ChatBoxSubmitMessage { .. })));
    }

    #[test]
    fn keymap_init_input_scope_catch_all_handles_char() {
        // Given an initialized keymap in Input scope.
        let keymap = init();
        let mut state = ratatui_which_key::WhichKeyState::new(keymap, Scope::Input);

        // When pressing 'a' (no explicit binding).
        let result = state.handle_key(key_event(Key::Char('a')));

        // Then catch_all returns ChatBoxInsertChar with 'a'.
        assert!(matches!(
            result,
            Some(Command::ChatBoxInsertChar {
                payload: ChatBoxInsertChar { ch: 'a' }
            })
        ));
    }

    #[test]
    fn keymap_init_normal_scope_esc_quits() {
        // Given an initialized keymap.
        let keymap = init();
        let mut state = ratatui_which_key::WhichKeyState::new(keymap, Scope::Normal);

        // When navigating Normal scope with Esc.
        let result = state.handle_key(key_event(Key::Esc));

        // Then returns AppQuit.
        assert!(matches!(result, Some(Command::AppQuit)));
    }
}
