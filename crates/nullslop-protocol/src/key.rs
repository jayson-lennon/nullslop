//! Key representation for keyboard events.
//!
//! Backend-agnostic key types that abstract away the underlying
//! terminal library (crossterm, termion, etc.).

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
