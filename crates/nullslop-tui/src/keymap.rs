//! Keymap configuration and initialization.
//!
//! Defines the key categories and builds the keymap with all scope bindings.
//! Binds keys to [`Command`](nullslop_protocol::Command) variants. Parameterized on
//! [`nullslop_protocol::KeyEvent`] so the keymap works in both TUI and headless modes.

use derive_more::Display;
use nullslop_protocol::chat_input::{InsertChar, SubmitMessage};
use nullslop_protocol::provider_picker::PickerInsertChar;
use nullslop_protocol::system::SetMode;
use nullslop_protocol::tab::SwitchTab;
use nullslop_protocol::{Command, Key, KeyEvent, Mode, SessionId, TabDirection};
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
#[rustfmt::skip]
pub fn init() -> Keymap<KeyEvent, Scope, Command, KeyCategory> {
    let mut keymap = Keymap::new();

    keymap
        // Normal scope: navigation and commands
        .scope(Scope::Normal, |b| {
            b
            .bind("i", Command::SetMode { payload: SetMode { mode: Mode::Input } }, KeyCategory::General)
            .bind("q", Command::Quit, KeyCategory::General)
            .bind("<c-c>", Command::Quit, KeyCategory::General)
            .bind("?", Command::ToggleWhichKey, KeyCategory::General)
            .bind("<c-e>", Command::EditInput, KeyCategory::Input)
            .bind("<c-h>", Command::SwitchTab { payload: SwitchTab { direction: TabDirection::Prev } }, KeyCategory::General)
            .bind("<c-l>", Command::SwitchTab { payload: SwitchTab { direction: TabDirection::Next } }, KeyCategory::General)
            .bind("<c-u>", Command::ScrollUp, KeyCategory::General)
        .describe_group("g", "general")
            .describe_group("gm", "model")
            .bind("gms", Command::SetMode { payload: SetMode { mode: Mode::Picker } }, KeyCategory::General)
            // .bind("gmr", Command::RefreshModels, KeyCategory::General) // TODO: re-enable when type is defined
            .bind("<c-d>", Command::ScrollDown, KeyCategory::General);
        })
        // Input scope: typing into the input buffer
        .scope(Scope::Input, |b| {
            b.bind("<enter>", Command::SubmitMessage { payload: SubmitMessage { session_id: SessionId::new(), text: String::new() } }, KeyCategory::Input)
            .bind("<s-enter>", Command::InsertChar { payload: InsertChar { ch: '\n' } }, KeyCategory::Input)
            .bind("<c-enter>", Command::InsertChar { payload: InsertChar { ch: '\n' } }, KeyCategory::Input)
            .bind("<esc>", Command::SetMode { payload: SetMode { mode: Mode::Normal } }, KeyCategory::General)
            .bind("<c-c>", Command::Interrupt, KeyCategory::General)
            .bind("<c-e>", Command::EditInput, KeyCategory::Input)
            .bind("<f1>", Command::ToggleWhichKey, KeyCategory::General)
            .bind("<backspace>", Command::DeleteGrapheme, KeyCategory::Input)
            .bind("<left>", Command::MoveCursorLeft, KeyCategory::Input)
            .bind("<right>", Command::MoveCursorRight, KeyCategory::Input)
            .bind("<home>", Command::MoveCursorToStart, KeyCategory::Input)
            .bind("<end>", Command::MoveCursorToEnd, KeyCategory::Input)
            .bind("<delete>", Command::DeleteGraphemeForward, KeyCategory::Input)
            .bind("<c-left>", Command::MoveCursorWordLeft, KeyCategory::Input)
            .bind("<c-right>", Command::MoveCursorWordRight, KeyCategory::Input)
            .bind("<up>", Command::MoveCursorUp, KeyCategory::Input)
            .bind("<down>", Command::MoveCursorDown, KeyCategory::Input)
            .bind("<c-u>", Command::ScrollUp, KeyCategory::General)
            .bind("<c-d>", Command::ScrollDown, KeyCategory::General)
            .catch_all(|key: KeyEvent| {
                if let Key::Char(c) = key.key {
                    Some(Command::InsertChar {
                        payload: InsertChar { ch: c },
                    })
                } else {
                    None
                }
            });
        });

    // Picker scope: filter input and provider selection
    keymap
        .scope(Scope::Picker, |b| {
            b.bind("<esc>", Command::SetMode { payload: SetMode { mode: Mode::Normal } }, KeyCategory::General)
            .bind("<enter>", Command::PickerConfirm, KeyCategory::General)
            .bind("<up>", Command::PickerMoveUp, KeyCategory::General)
            .bind("<down>", Command::PickerMoveDown, KeyCategory::General)
            .bind("<left>", Command::PickerMoveCursorLeft, KeyCategory::Input)
            .bind("<right>", Command::PickerMoveCursorRight, KeyCategory::Input)
            .bind("<backspace>", Command::PickerBackspace, KeyCategory::Input)
            .catch_all(|key: KeyEvent| {
                if let Key::Char(c) = key.key {
                    Some(Command::PickerInsertChar {
                        payload: PickerInsertChar { ch: c },
                    })
                } else {
                    None
                }
            });
        });

    keymap
}

// No tests needed — keymap bindings are exercised end-to-end through the
// TUI app integration tests in `app.rs` (key press → command dispatch).

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scope::Scope;
    use nullslop_protocol::Modifiers;
    use ratatui_which_key::Key as _;

    #[test]
    fn g_shows_in_which_key_with_general_description() {
        // Given the keymap.
        let keymap = init();

        // When getting bindings for Normal scope.
        let bindings = keymap.bindings_for_scope(Scope::Normal);

        // Find the 'g' binding across all groups.
        let g_binding = bindings
            .iter()
            .flat_map(|g| g.bindings.iter())
            .find(|b| b.key.display() == "g");

        // Then 'g' is present with description "general".
        assert!(
            g_binding.is_some(),
            "'g' binding should appear in Normal scope"
        );
        assert_eq!(g_binding.unwrap().description, "general");
    }

    #[test]
    fn gms_produces_set_mode_picker() {
        // Given the keymap.
        let keymap = init();

        // When looking up 'g' then 'm' then 's'.
        let g_key = KeyEvent {
            key: Key::Char('g'),
            modifiers: Modifiers::none(),
        };
        let m_key = KeyEvent {
            key: Key::Char('m'),
            modifiers: Modifiers::none(),
        };
        let s_key = KeyEvent {
            key: Key::Char('s'),
            modifiers: Modifiers::none(),
        };

        let node = keymap.get_node_at_path(&[g_key, m_key, s_key]);

        // Then it's a leaf with the SetMode Picker command.
        assert!(node.is_some());
        if let Some(ratatui_which_key::KeyNode::Leaf(entries)) = node {
            let entry = entries.iter().find(|e| e.scope == Scope::Normal);
            assert!(entry.is_some());
            let cmd = &entry.unwrap().action;
            assert!(
                matches!(cmd, Command::SetMode { payload } if payload.mode == Mode::Picker),
                "expected SetMode Picker, got {cmd:?}"
            );
        } else {
            panic!("Expected leaf node for 'gms'");
        }
    }

    #[test]
    fn gms_produces_picker_mode_command() {
        // Given the keymap.
        let keymap = init();

        // When looking up 'g' then 'm' then 's'.
        let g_key = KeyEvent {
            key: Key::Char('g'),
            modifiers: Modifiers::none(),
        };
        let m_key = KeyEvent {
            key: Key::Char('m'),
            modifiers: Modifiers::none(),
        };
        let s_key = KeyEvent {
            key: Key::Char('s'),
            modifiers: Modifiers::none(),
        };

        let node = keymap.get_node_at_path(&[g_key, m_key, s_key]);

        // Then it's a leaf with the SetMode command.
        assert!(node.is_some());
        if let Some(ratatui_which_key::KeyNode::Leaf(entries)) = node {
            let entry = entries.iter().find(|e| e.scope == Scope::Normal);
            assert!(entry.is_some());
            let cmd = &entry.unwrap().action;
            assert!(
                matches!(cmd, Command::SetMode { .. }),
                "expected SetMode, got {cmd:?}"
            );
        } else {
            panic!("Expected leaf node for 'gms'");
        }
    }
}
