//! Key representation for keyboard events.
//!
//! Backend-agnostic key types that decouple key handling
//! from any specific terminal library.

use serde::{Deserialize, Serialize};

/// Keyboard key.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Key {
    /// A character key.
    Char(char),
    /// Enter key.
    Enter,
    /// Escape key.
    Esc,
    /// Tab key.
    Tab,
    /// Backspace key.
    Backspace,
    /// Up arrow.
    Up,
    /// Down arrow.
    Down,
    /// Left arrow.
    Left,
    /// Right arrow.
    Right,
    /// Home key.
    Home,
    /// End key.
    End,
    /// Page up key.
    PageUp,
    /// Page down key.
    PageDown,
    /// Function key (F1–F12).
    F(u8),
}

#[cfg(feature = "which-key")]
impl ratatui_which_key::Key for KeyEvent {
    fn display(&self) -> String {
        if self.modifiers.ctrl
            && let Key::Char(c) = self.key
        {
            return format!("<C-{}>", c.to_ascii_lowercase());
        }

        match self.key {
            Key::Char(' ') => "Space".to_string(),
            Key::Char(c) => c.to_string(),
            Key::Tab => "Tab".to_string(),
            Key::Enter => "Enter".to_string(),
            Key::Backspace => "Backspace".to_string(),
            Key::Esc => "Esc".to_string(),
            Key::Up => "↑".to_string(),
            Key::Down => "↓".to_string(),
            Key::Left => "←".to_string(),
            Key::Right => "→".to_string(),
            Key::Home => "Home".to_string(),
            Key::End => "End".to_string(),
            Key::PageUp => "PageUp".to_string(),
            Key::PageDown => "PageDown".to_string(),
            Key::F(n) => format!("F{n}"),
        }
    }

    fn is_backspace(&self) -> bool {
        matches!(self.key, Key::Backspace)
    }

    fn space() -> Self {
        KeyEvent {
            key: Key::Char(' '),
            modifiers: Modifiers::none(),
        }
    }

    fn from_char(c: char) -> Option<Self> {
        Some(KeyEvent {
            key: Key::Char(c),
            modifiers: Modifiers::none(),
        })
    }

    /// Parse a key from a special name string.
    ///
    /// Supports control-modified keys via the `c-` prefix (e.g. `"c-x"` → Ctrl+X),
    /// as well as named keys like `"tab"`, `"enter"`, `"escape"`, arrow keys,
    /// function keys (`"f1"`–`"f12"`), and symbolic aliases (`"lt"` → `<`, `"gt"` → `>`).
    ///
    /// Matching is case-insensitive.
    ///
    /// # Examples
    ///
    /// - `"c-x"` → Ctrl+X
    /// - `"tab"` → Tab
    /// - `"f5"` → F5
    /// - `"lt"` → <
    fn from_special_name(name: &str) -> Option<Self> {
        let lower = name.to_ascii_lowercase();

        if lower.starts_with("c-") && lower.len() == 3 {
            let c = lower.chars().nth(2)?;
            return Some(KeyEvent {
                key: Key::Char(c),
                modifiers: Modifiers::ctrl(),
            });
        }

        let key = match lower.as_str() {
            "tab" => Key::Tab,
            "enter" => Key::Enter,
            "bs" | "backspace" => Key::Backspace,
            "esc" | "escape" => Key::Esc,
            "up" => Key::Up,
            "down" => Key::Down,
            "left" => Key::Left,
            "right" => Key::Right,
            "home" => Key::Home,
            "end" => Key::End,
            "pgup" | "pageup" => Key::PageUp,
            "pgdn" | "pagedown" => Key::PageDown,
            "space" => Key::Char(' '),
            "lt" => Key::Char('<'),
            "gt" => Key::Char('>'),
            s if s.starts_with('f') && s.len() > 1 => {
                let num: u8 = s[1..].parse().ok()?;
                if !(1..=12).contains(&num) {
                    return None;
                }
                Key::F(num)
            }
            _ => return None,
        };

        Some(KeyEvent {
            key,
            modifiers: Modifiers::none(),
        })
    }
}

/// Keyboard modifier flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Modifiers {
    /// Control key held.
    pub ctrl: bool,
    /// Alt key held.
    pub alt: bool,
    /// Shift key held.
    pub shift: bool,
}

impl Modifiers {
    /// Create a modifiers with no flags set.
    #[must_use]
    pub fn none() -> Self {
        Self {
            ctrl: false,
            alt: false,
            shift: false,
        }
    }

    /// Create a modifiers with only ctrl set.
    #[must_use]
    pub fn ctrl() -> Self {
        Self {
            ctrl: true,
            alt: false,
            shift: false,
        }
    }

    /// Create a modifiers with only alt set.
    #[must_use]
    pub fn alt() -> Self {
        Self {
            ctrl: false,
            alt: true,
            shift: false,
        }
    }

    /// Create a modifiers with only shift set.
    #[must_use]
    pub fn shift() -> Self {
        Self {
            ctrl: false,
            alt: false,
            shift: true,
        }
    }

    /// Returns `true` if no modifier flags are set.
    #[must_use]
    pub fn is_none(&self) -> bool {
        !self.ctrl && !self.alt && !self.shift
    }
}

/// A keyboard event with key and modifiers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyEvent {
    /// The key that was pressed.
    pub key: Key,
    /// Modifier keys held at the time.
    pub modifiers: Modifiers,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifiers_none_is_all_false() {
        // Given a new modifiers with none().
        let mods = Modifiers::none();

        // When inspecting flags.

        // Then all flags are false.
        assert!(!mods.ctrl);
        assert!(!mods.alt);
        assert!(!mods.shift);
        assert!(mods.is_none());
    }

    #[rstest::rstest]
    #[case::char(Key::Char('a'))]
    #[case::enter(Key::Enter)]
    #[case::esc(Key::Esc)]
    #[case::tab(Key::Tab)]
    #[case::backspace(Key::Backspace)]
    #[case::up(Key::Up)]
    #[case::down(Key::Down)]
    #[case::left(Key::Left)]
    #[case::right(Key::Right)]
    #[case::home(Key::Home)]
    #[case::end(Key::End)]
    #[case::page_up(Key::PageUp)]
    #[case::page_down(Key::PageDown)]
    #[case::f1(Key::F(1))]
    #[case::f12(Key::F(12))]
    fn key_serialization_roundtrip(#[case] key: Key) {
        // Given a key variant.
        let json = serde_json::to_string(&key).expect("serialize key");

        // When deserializing.
        let back: Key = serde_json::from_str(&json).expect("deserialize key");

        // Then it matches the original.
        assert_eq!(back, key);
    }

    #[test]
    fn key_event_serialization_roundtrip() {
        // Given a KeyEvent with modifiers.
        let event = KeyEvent {
            key: Key::Char('x'),
            modifiers: Modifiers::ctrl(),
        };

        // When serialized and deserialized.
        let json = serde_json::to_string(&event).expect("serialize");
        let back: KeyEvent = serde_json::from_str(&json).expect("deserialize");

        // Then it matches the original.
        assert_eq!(back, event);
    }
}
