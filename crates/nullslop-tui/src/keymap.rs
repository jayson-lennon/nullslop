//! Keymap configuration and initialization.
//!
//! Defines the key categories and builds the keymap with all scope bindings.
//! Binds keys to [`Command`](nullslop_protocol::Command) variants. Parameterized on
//! [`nullslop_protocol::KeyEvent`] so the keymap works in both TUI and headless modes.

use derive_more::Display;
use nullslop_protocol::chat_input::{InsertChar, SubmitMessage};
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
            b.bind("i", Command::SetMode { payload: SetMode { mode: Mode::Input } }, KeyCategory::General)
            .bind("q", Command::Quit, KeyCategory::General)
            .bind("?", Command::ToggleWhichKey, KeyCategory::General)
            .bind("<c-e>", Command::EditInput, KeyCategory::Input)
            .bind("<c-h>", Command::SwitchTab { payload: SwitchTab { direction: TabDirection::Prev } }, KeyCategory::General)
            .bind("<c-l>", Command::SwitchTab { payload: SwitchTab { direction: TabDirection::Next } }, KeyCategory::General)
            .bind("<c-u>", Command::ScrollUp, KeyCategory::General)
            .bind("<c-d>", Command::ScrollDown, KeyCategory::General);
        })
        // Input scope: typing into the input buffer
        .scope(Scope::Input, |b| {
            b.bind("<enter>", Command::SubmitMessage { payload: SubmitMessage { session_id: SessionId::new(), text: String::new() } }, KeyCategory::Input)
            .bind("<s-enter>", Command::InsertChar { payload: InsertChar { ch: '\n' } }, KeyCategory::Input)
            .bind("<c-enter>", Command::InsertChar { payload: InsertChar { ch: '\n' } }, KeyCategory::Input)
            .bind("<esc>", Command::SetMode { payload: SetMode { mode: Mode::Normal } }, KeyCategory::General)
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

    keymap
}

// No tests needed — keymap bindings are exercised end-to-end through the
// TUI app integration tests in `app.rs` (key press → command dispatch).
