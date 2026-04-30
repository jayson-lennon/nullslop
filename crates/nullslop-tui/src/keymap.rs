//! Keymap configuration and initialization.
//!
//! Defines the key categories and builds the keymap with all scope bindings.
//! Binds keys to [`Command`](nullslop_protocol::Command) variants. Parameterized on
//! [`nullslop_protocol::KeyEvent`] so the keymap works in both TUI and headless modes.

use derive_more::Display;
use nullslop_protocol::command::{
    AppSetMode, AppSwitchTab, ChatBoxInsertChar, ChatBoxSubmitMessage,
};
use nullslop_protocol::{Command, Mode, TabDirection};
use nullslop_protocol::{Key, KeyEvent};
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
#[allow(clippy::too_many_lines)]
pub fn init() -> Keymap<KeyEvent, Scope, Command, KeyCategory> {
    let mut keymap = Keymap::new();

    keymap
        // Normal scope: navigation and commands
        .scope(Scope::Normal, |b| {
            b.bind(
                "i",
                Command::AppSetMode {
                    payload: AppSetMode { mode: Mode::Input },
                },
                KeyCategory::General,
            )
            .bind("q", Command::AppQuit, KeyCategory::General)
            .bind("?", Command::AppToggleWhichKey, KeyCategory::General)
            .bind("<c-e>", Command::AppEditInput, KeyCategory::Input)
            .bind(
                "<c-h>",
                Command::AppSwitchTab {
                    payload: AppSwitchTab {
                        direction: TabDirection::Prev,
                    },
                },
                KeyCategory::General,
            )
            .bind(
                "<c-l>",
                Command::AppSwitchTab {
                    payload: AppSwitchTab {
                        direction: TabDirection::Next,
                    },
                },
                KeyCategory::General,
            );
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
            .bind("<f1>", Command::AppToggleWhichKey, KeyCategory::General)
            .bind(
                "<backspace>",
                Command::ChatBoxDeleteGrapheme,
                KeyCategory::Input,
            )
            .bind("<left>", Command::ChatBoxMoveCursorLeft, KeyCategory::Input)
            .bind(
                "<right>",
                Command::ChatBoxMoveCursorRight,
                KeyCategory::Input,
            )
            .bind(
                "<home>",
                Command::ChatBoxMoveCursorToStart,
                KeyCategory::Input,
            )
            .bind("<end>", Command::ChatBoxMoveCursorToEnd, KeyCategory::Input)
            .bind(
                "<delete>",
                Command::ChatBoxDeleteGraphemeForward,
                KeyCategory::Input,
            )
            .bind(
                "<c-left>",
                Command::ChatBoxMoveCursorWordLeft,
                KeyCategory::Input,
            )
            .bind(
                "<c-right>",
                Command::ChatBoxMoveCursorWordRight,
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

// No tests needed — keymap bindings are exercised end-to-end through the
// TUI app integration tests in `app.rs` (key press → command dispatch).
