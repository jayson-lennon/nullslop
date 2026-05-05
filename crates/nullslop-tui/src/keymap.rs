//! Keymap configuration and initialization.
//!
//! Defines the key categories and builds the keymap with all scope bindings.
//! Binds keys to [`Command`](nullslop_protocol::Command) variants. Parameterized on
//! [`nullslop_protocol::KeyEvent`] so the keymap works in both TUI and headless modes.

use crossterm::event::{self, MouseEventKind};
use derive_more::Display;
use nullslop_protocol::chat_input::{InsertChar, SubmitMessage};
use nullslop_protocol::provider_picker::PickerInsertChar;
use nullslop_protocol::system::SetMode;
use nullslop_protocol::tab::SwitchTab;
use nullslop_protocol::{Command, Key, KeyEvent, Mode, SessionId, TabDirection};
use ratatui_which_key::CrosstermKeymapExt;
use ratatui_which_key::Keymap;

use crate::scope::Scope;

/// Categories for keybinding grouping in the which-key popup.
///
/// Each variant becomes a section header when displaying available shortcuts.
#[derive(Display, Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCategory {
    /// App-level control: quit, interrupt, help.
    General,
    /// Navigation: scrolling, tab switching, picker movement.
    Navigation,
    /// Model management: model picker, model refresh.
    Model,
    /// Text editing: cursor movement, insertion, deletion, mode entry.
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
            // General — app control
            .bind("q", Command::Quit, KeyCategory::General)
            .bind("j", Command::DashboardSelectDown, KeyCategory::General)
            .bind("k", Command::DashboardSelectUp, KeyCategory::General)
            .bind("<c-c>", Command::Quit, KeyCategory::General)
            .bind("?", Command::ToggleWhichKey, KeyCategory::General)
            // Input — enter input mode
            .bind("i", Command::SetMode { payload: SetMode { mode: Mode::Input } }, KeyCategory::Input)
            // Navigation — scrolling and tab switching
            .bind("k", Command::ScrollLineUp, KeyCategory::Navigation)
            .bind("j", Command::ScrollLineDown, KeyCategory::Navigation)
            .bind("<c-u>", Command::ScrollUp, KeyCategory::Navigation)
            .bind("<c-d>", Command::ScrollDown, KeyCategory::Navigation)
            .bind("<tab>", Command::SwitchTab { payload: SwitchTab { direction: TabDirection::Next } }, KeyCategory::Navigation)
            .bind("<s-tab>", Command::SwitchTab { payload: SwitchTab { direction: TabDirection::Prev } }, KeyCategory::Navigation)
            // Input — external editor
            .bind("<c-e>", Command::EditInput, KeyCategory::Input)
            // g prefix — general commands and model management
            .describe_group_with_category("g", "general", KeyCategory::General)
            .describe_group_with_category("gm", "model", KeyCategory::Model)
            .bind("gg", Command::ScrollToTop, KeyCategory::Navigation)
            .bind("G", Command::ScrollToBottom, KeyCategory::Navigation)
            .bind("gmp", Command::SetMode { payload: SetMode { mode: Mode::Picker } }, KeyCategory::Model)
            .bind("gmr", Command::RefreshModels, KeyCategory::Model);
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
            .bind("<c-u>", Command::ScrollUp, KeyCategory::Navigation)
            .bind("<c-d>", Command::ScrollDown, KeyCategory::Navigation)
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
        .scope(Scope::Picker, |b| {
            b.bind("<esc>", Command::SetMode { payload: SetMode { mode: Mode::Normal } }, KeyCategory::General)
            .bind("<enter>", Command::PickerConfirm, KeyCategory::Model)
            .bind("<up>", Command::PickerMoveUp, KeyCategory::Navigation)
            .bind("<down>", Command::PickerMoveDown, KeyCategory::Navigation)
            .bind("<left>", Command::PickerMoveCursorLeft, KeyCategory::Input)
            .bind("<right>", Command::PickerMoveCursorRight, KeyCategory::Input)
            .bind("<backspace>", Command::PickerBackspace, KeyCategory::Input)
            .bind("<c-r>", Command::RefreshModels, KeyCategory::Model)
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

    keymap.on_mouse(|mouse: event::MouseEvent, _scope: &Scope| {
        match mouse.kind {
            MouseEventKind::ScrollUp => Some(Command::MouseScrollUp),
            MouseEventKind::ScrollDown => Some(Command::MouseScrollDown),
            _ => None,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scope::Scope;
    use nullslop_protocol::Modifiers;
    use ratatui_which_key::Key as _;

    // --- Normal scope: key sequence resolution ---

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
    fn gmp_produces_set_mode_picker() {
        // Given the keymap.
        let keymap = init();

        // When looking up 'g' then 'm' then 'p'.
        let g_key = KeyEvent {
            key: Key::Char('g'),
            modifiers: Modifiers::none(),
        };
        let m_key = KeyEvent {
            key: Key::Char('m'),
            modifiers: Modifiers::none(),
        };
        let p_key = KeyEvent {
            key: Key::Char('p'),
            modifiers: Modifiers::none(),
        };

        let node = keymap.get_node_at_path(&[g_key, m_key, p_key]);

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
            panic!("Expected leaf node for 'gmp'");
        }
    }

    #[test]
    fn gmr_produces_refresh_models_command() {
        // Given the keymap.
        let keymap = init();

        // When looking up 'g' then 'm' then 'r'.
        let g_key = KeyEvent {
            key: Key::Char('g'),
            modifiers: Modifiers::none(),
        };
        let m_key = KeyEvent {
            key: Key::Char('m'),
            modifiers: Modifiers::none(),
        };
        let r_key = KeyEvent {
            key: Key::Char('r'),
            modifiers: Modifiers::none(),
        };

        let node = keymap.get_node_at_path(&[g_key, m_key, r_key]);

        // Then it's a leaf with the RefreshModels command.
        assert!(node.is_some());
        if let Some(ratatui_which_key::KeyNode::Leaf(entries)) = node {
            let entry = entries.iter().find(|e| e.scope == Scope::Normal);
            assert!(entry.is_some());
            let cmd = &entry.unwrap().action;
            assert!(
                matches!(cmd, Command::RefreshModels),
                "expected RefreshModels, got {cmd:?}"
            );
        } else {
            panic!("Expected leaf node for 'gmr'");
        }
    }

    // --- New bindings: j/k line scroll, gg/G scroll to top/bottom ---

    #[test]
    fn j_produces_scroll_line_down() {
        // Given the keymap.
        let keymap = init();

        // When looking up 'j'.
        let j_key = KeyEvent {
            key: Key::Char('j'),
            modifiers: Modifiers::none(),
        };
        let node = keymap.get_node_at_path(&[j_key]);

        // Then it's a leaf with ScrollLineDown.
        assert!(node.is_some());
        if let Some(ratatui_which_key::KeyNode::Leaf(entries)) = node {
            let entry = entries.iter().find(|e| e.scope == Scope::Normal);
            assert!(entry.is_some());
            assert!(matches!(entry.unwrap().action, Command::ScrollLineDown));
        } else {
            panic!("Expected leaf node for 'j'");
        }
    }

    #[test]
    fn k_produces_scroll_line_up() {
        // Given the keymap.
        let keymap = init();

        // When looking up 'k'.
        let k_key = KeyEvent {
            key: Key::Char('k'),
            modifiers: Modifiers::none(),
        };
        let node = keymap.get_node_at_path(&[k_key]);

        // Then it's a leaf with ScrollLineUp.
        assert!(node.is_some());
        if let Some(ratatui_which_key::KeyNode::Leaf(entries)) = node {
            let entry = entries.iter().find(|e| e.scope == Scope::Normal);
            assert!(entry.is_some());
            assert!(matches!(entry.unwrap().action, Command::ScrollLineUp));
        } else {
            panic!("Expected leaf node for 'k'");
        }
    }

    #[test]
    fn gg_produces_scroll_to_top() {
        // Given the keymap.
        let keymap = init();

        // When looking up 'g' then 'g'.
        let g_key = KeyEvent {
            key: Key::Char('g'),
            modifiers: Modifiers::none(),
        };
        let node = keymap.get_node_at_path(&[g_key.clone(), g_key]);

        // Then it's a leaf with ScrollToTop.
        assert!(node.is_some());
        if let Some(ratatui_which_key::KeyNode::Leaf(entries)) = node {
            let entry = entries.iter().find(|e| e.scope == Scope::Normal);
            assert!(entry.is_some());
            assert!(matches!(entry.unwrap().action, Command::ScrollToTop));
        } else {
            panic!("Expected leaf node for 'gg'");
        }
    }

    #[test]
    fn uppercase_g_produces_scroll_to_bottom() {
        // Given the keymap.
        let keymap = init();

        // When looking up 'G' (uppercase).
        let g_key = KeyEvent {
            key: Key::Char('G'),
            modifiers: Modifiers::none(),
        };
        let node = keymap.get_node_at_path(&[g_key]);

        // Then it's a leaf with ScrollToBottom.
        assert!(node.is_some());
        if let Some(ratatui_which_key::KeyNode::Leaf(entries)) = node {
            let entry = entries.iter().find(|e| e.scope == Scope::Normal);
            assert!(entry.is_some());
            assert!(matches!(entry.unwrap().action, Command::ScrollToBottom));
        } else {
            panic!("Expected leaf node for 'G'");
        }
    }

    // --- Tab switching: Tab/Shift+Tab ---

    #[test]
    fn tab_produces_switch_tab_next() {
        // Given the keymap.
        let keymap = init();

        // When looking up '<tab>'.
        let tab_key = KeyEvent {
            key: Key::Tab,
            modifiers: Modifiers::none(),
        };
        let node = keymap.get_node_at_path(&[tab_key]);

        // Then it's a leaf with SwitchTab Next.
        assert!(node.is_some());
        if let Some(ratatui_which_key::KeyNode::Leaf(entries)) = node {
            let entry = entries.iter().find(|e| e.scope == Scope::Normal);
            assert!(entry.is_some());
            assert!(
                matches!(&entry.unwrap().action, Command::SwitchTab { payload } if payload.direction == TabDirection::Next),
                "expected SwitchTab Next"
            );
        } else {
            panic!("Expected leaf node for '<tab>'");
        }
    }

    #[test]
    fn shift_tab_produces_switch_tab_prev() {
        // Given the keymap.
        let keymap = init();

        // When looking up '<s-tab>'.
        let stab_key = KeyEvent {
            key: Key::Tab,
            modifiers: Modifiers::shift(),
        };
        let node = keymap.get_node_at_path(&[stab_key]);

        // Then it's a leaf with SwitchTab Prev.
        assert!(node.is_some());
        if let Some(ratatui_which_key::KeyNode::Leaf(entries)) = node {
            let entry = entries.iter().find(|e| e.scope == Scope::Normal);
            assert!(entry.is_some());
            assert!(
                matches!(&entry.unwrap().action, Command::SwitchTab { payload } if payload.direction == TabDirection::Prev),
                "expected SwitchTab Prev"
            );
        } else {
            panic!("Expected leaf node for '<s-tab>'");
        }
    }

    // --- Category assignments ---

    #[test]
    fn normal_scope_general_category_has_quit_and_help() {
        // Given the keymap.
        let keymap = init();

        // When getting bindings grouped by category for Normal scope.
        let groups = keymap.bindings_for_scope(Scope::Normal);
        let general = groups.iter().find(|g| g.category == "General");

        // Then the General group contains quit and help bindings.
        assert!(general.is_some(), "General category should exist");
        let descs: Vec<&str> = general.unwrap().bindings.iter().map(|b| b.description.as_str()).collect();
        assert!(descs.contains(&"quit"), "General should contain quit");
        assert!(descs.contains(&"toggle which-key"), "General should contain toggle which-key");
    }

    #[test]
    fn normal_scope_mode_category_contains_set_mode_input() {
        // Given the keymap.
        let keymap = init();

        // When getting bindings grouped by category for Normal scope.
        let groups = keymap.bindings_for_scope(Scope::Normal);
        let input = groups.iter().find(|g| g.category == "Input");

        // Then the Input group exists and contains 'i' → set mode input.
        assert!(input.is_some(), "Input category should exist");
        let descs: Vec<&str> = input.unwrap().bindings.iter().map(|b| b.description.as_str()).collect();
        assert!(descs.iter().any(|d| d.contains("input")), "Input should contain set mode input");
    }

    #[test]
    fn normal_scope_navigation_category_has_scroll_and_tab() {
        // Given the keymap.
        let keymap = init();

        // When getting bindings grouped by category for Normal scope.
        let groups = keymap.bindings_for_scope(Scope::Normal);
        let nav = groups.iter().find(|g| g.category == "Navigation");

        // Then the Navigation group contains scroll and tab bindings.
        assert!(nav.is_some(), "Navigation category should exist");
        let descs: Vec<&str> = nav.unwrap().bindings.iter().map(|b| b.description.as_str()).collect();
        assert!(descs.contains(&"scroll up"), "Navigation should contain scroll up");
        assert!(descs.contains(&"scroll down"), "Navigation should contain scroll down");
        assert!(descs.iter().any(|d| d.contains("tab")), "Navigation should contain tab switch");
    }

    #[test]
    fn gm_prefix_appears_under_model_category() {
        // Given the keymap.
        let keymap = init();

        // When navigating into the 'g' prefix in Normal scope.
        let g_key = KeyEvent {
            key: Key::Char('g'),
            modifiers: Modifiers::none(),
        };
        let children = keymap
            .get_children_at_path(&[g_key], &Scope::Normal)
            .expect("g prefix should have children");

        // Then 'm' is one of the children with description "model".
        let m_child = children.iter().find(|(k, _)| k.display() == "m");
        assert!(m_child.is_some(), "'m' should be a child of 'g'");
        assert_eq!(m_child.unwrap().1, "model");
    }

    #[test]
    fn g_prefix_appears_under_general_category() {
        // Given the keymap.
        let keymap = init();

        // When getting bindings grouped by category for Normal scope.
        let groups = keymap.bindings_for_scope(Scope::Normal);
        let general = groups.iter().find(|g| g.category == "General");

        // Then the General group contains 'g' with description "general".
        assert!(general.is_some(), "General category should exist");
        let g_binding = general
            .unwrap()
            .bindings
            .iter()
            .find(|b| b.key.display() == "g");
        assert!(
            g_binding.is_some(),
            "General category should contain 'g' prefix"
        );
        assert_eq!(g_binding.unwrap().description, "general");
    }

    #[test]
    fn input_scope_escape_appears_under_general_category() {
        // Given the keymap.
        let keymap = init();

        // When getting bindings grouped by category for Input scope.
        let groups = keymap.bindings_for_scope(Scope::Input);
        let general = groups.iter().find(|g| g.category == "General");

        // Then the General group contains '<esc>' → set mode normal.
        assert!(general.is_some(), "General category should exist");
        let descs: Vec<&str> = general
            .unwrap()
            .bindings
            .iter()
            .map(|b| b.description.as_str())
            .collect();
        assert!(
            descs.iter().any(|d| d.contains("normal")),
            "General should contain set mode normal, found: {descs:?}"
        );
    }
}
