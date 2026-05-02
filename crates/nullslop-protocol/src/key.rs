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
    /// Delete key (forward delete).
    Delete,
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

        let base = match self.key {
            Key::Char(' ') => "Space".to_owned(),
            Key::Char(c) => c.to_string(),
            Key::Tab => "Tab".to_owned(),
            Key::Enter => "Enter".to_owned(),
            Key::Backspace => "Backspace".to_owned(),
            Key::Esc => "Esc".to_owned(),
            Key::Up => "↑".to_owned(),
            Key::Down => "↓".to_owned(),
            Key::Left => "←".to_owned(),
            Key::Right => "→".to_owned(),
            Key::Home => "Home".to_owned(),
            Key::End => "End".to_owned(),
            Key::PageUp => "PageUp".to_owned(),
            Key::PageDown => "PageDown".to_owned(),
            Key::Delete => "Delete".to_owned(),
            Key::F(n) => format!("F{n}"),
        };

        match (self.modifiers.shift, self.modifiers.ctrl) {
            (true, false) => format!("S-{base}"),
            (false, true) => format!("C-{base}"),
            (true, true) => format!("C-S-{base}"),
            _ => base,
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
}

impl KeyEvent {
    /// Parse a key notation string into a `KeyEvent`.
    ///
    /// Supports modifier-prefixed forms: `c-` for Ctrl and `s-` for Shift.
    /// Modifiers apply to both named keys and single characters.
    ///
    /// Named keys: `"tab"`, `"enter"`, `"escape"`, arrow keys,
    /// function keys (`"f1"`–`"f12"`), and symbolic aliases (`"lt"` → `<`, `"gt"` → `>`).
    ///
    /// Matching is case-insensitive.
    ///
    /// # Examples
    ///
    /// - `"c-x"` → Ctrl+X
    /// - `"s-enter"` → Shift+Enter
    /// - `"c-enter"` → Ctrl+Enter
    /// - `"tab"` → Tab
    /// - `"f5"` → F5
    /// - `"lt"` → <
    #[cfg(feature = "which-key")]
    pub fn parse_notation(name: &str) -> Option<Self> {
        let lower = name.to_ascii_lowercase();

        let (modifiers, rest) = if let Some(stripped) = lower.strip_prefix("s-") {
            (Modifiers::shift(), stripped)
        } else if let Some(stripped) = lower.strip_prefix("c-") {
            (Modifiers::ctrl(), stripped)
        } else {
            (Modifiers::none(), lower.as_str())
        };

        let key = parse_key_name(rest)?;

        Some(KeyEvent { key, modifiers })
    }
}

/// Parse a lower-case key name string into a [`Key`].
///
/// Handles named keys (`"tab"`, `"enter"`, …), function keys (`"f1"`–`"f12"`),
/// symbolic aliases (`"lt"`, `"gt"`, `"space"`), and bare single characters.
fn parse_key_name(name: &str) -> Option<Key> {
    match name {
        "tab" => Some(Key::Tab),
        "enter" => Some(Key::Enter),
        "bs" | "backspace" => Some(Key::Backspace),
        "esc" | "escape" => Some(Key::Esc),
        "up" => Some(Key::Up),
        "down" => Some(Key::Down),
        "left" => Some(Key::Left),
        "right" => Some(Key::Right),
        "home" => Some(Key::Home),
        "end" => Some(Key::End),
        "pgup" | "pageup" => Some(Key::PageUp),
        "pgdn" | "pagedown" => Some(Key::PageDown),
        "delete" | "del" => Some(Key::Delete),
        "space" => Some(Key::Char(' ')),
        "lt" => Some(Key::Char('<')),
        "gt" => Some(Key::Char('>')),
        s if s.starts_with('f') && s.len() > 1 => {
            let num: u8 = s.get(1..)?.parse().ok()?;
            (1..=12).contains(&num).then_some(Key::F(num))
        }
        s if s.len() == 1 => Some(Key::Char(s.chars().next()?)),
        _ => None,
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
    #[case::delete(Key::Delete)]
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

    #[cfg(feature = "which-key")]
    #[test]
    fn parse_notation_s_enter_returns_shift_enter() {
        // Given the notation "s-enter".
        let result = KeyEvent::parse_notation("s-enter");

        // When parsing.
        let key_event = result.expect("should parse");

        // Then it is Shift+Enter.
        assert_eq!(key_event.key, Key::Enter);
        assert!(key_event.modifiers.shift);
        assert!(!key_event.modifiers.ctrl);
    }

    #[cfg(feature = "which-key")]
    #[test]
    fn parse_notation_c_enter_returns_ctrl_enter() {
        // Given the notation "c-enter".
        let result = KeyEvent::parse_notation("c-enter");

        // When parsing.
        let key_event = result.expect("should parse");

        // Then it is Ctrl+Enter.
        assert_eq!(key_event.key, Key::Enter);
        assert!(key_event.modifiers.ctrl);
        assert!(!key_event.modifiers.shift);
    }

    #[cfg(feature = "which-key")]
    #[test]
    fn parse_notation_enter_returns_unmodified() {
        // Given the notation "enter".
        let result = KeyEvent::parse_notation("enter");

        // When parsing.
        let key_event = result.expect("should parse");

        // Then it is plain Enter with no modifiers.
        assert_eq!(key_event.key, Key::Enter);
        assert!(key_event.modifiers.is_none());
    }

    // --- parse_key_name coverage (via parse_notation) ---

    #[cfg(feature = "which-key")]
    #[rstest::rstest]
    #[case::tab("tab", Key::Tab)]
    #[case::enter("enter", Key::Enter)]
    #[case::bs("bs", Key::Backspace)]
    #[case::backspace("backspace", Key::Backspace)]
    #[case::esc("esc", Key::Esc)]
    #[case::escape("escape", Key::Esc)]
    #[case::up("up", Key::Up)]
    #[case::down("down", Key::Down)]
    #[case::left("left", Key::Left)]
    #[case::right("right", Key::Right)]
    #[case::home("home", Key::Home)]
    #[case::end("end", Key::End)]
    #[case::pgup("pgup", Key::PageUp)]
    #[case::pageup("pageup", Key::PageUp)]
    #[case::pgdn("pgdn", Key::PageDown)]
    #[case::pagedown("pagedown", Key::PageDown)]
    #[case::delete("delete", Key::Delete)]
    #[case::del("del", Key::Delete)]
    #[case::space("space", Key::Char(' '))]
    #[case::lt("lt", Key::Char('<'))]
    #[case::gt("gt", Key::Char('>'))]
    fn parse_notation_named_keys(#[case] input: &str, #[case] expected: Key) {
        // Given a notation string for a named key.
        let result = KeyEvent::parse_notation(input);

        // When parsing.
        let key_event = result.expect("should parse");

        // Then the key matches with no modifiers.
        assert_eq!(key_event.key, expected);
        assert!(key_event.modifiers.is_none());
    }

    #[cfg(feature = "which-key")]
    #[rstest::rstest]
    #[case::f1("f1", 1)]
    #[case::f6("f6", 6)]
    #[case::f12("f12", 12)]
    fn parse_notation_function_keys(#[case] input: &str, #[case] num: u8) {
        // Given a function key notation.
        let result = KeyEvent::parse_notation(input);

        // When parsing.
        let key_event = result.expect("should parse");

        // Then it is the correct function key.
        assert_eq!(key_event.key, Key::F(num));
        assert!(key_event.modifiers.is_none());
    }

    #[cfg(feature = "which-key")]
    #[rstest::rstest]
    #[case::f0("f0")]
    #[case::f13("f13")]
    fn parse_notation_rejects_out_of_range_function_keys(#[case] input: &str) {
        // Given an out-of-range function key notation.
        let result = KeyEvent::parse_notation(input);

        // When parsing.

        // Then it returns None.
        assert!(result.is_none());
    }

    #[cfg(feature = "which-key")]
    #[test]
    fn parse_notation_single_char_returns_key_event() {
        // Given a single-character notation.
        let result = KeyEvent::parse_notation("a");

        // When parsing.
        let key_event = result.expect("should parse");

        // Then it is Char('a') with no modifiers.
        assert_eq!(key_event.key, Key::Char('a'));
        assert!(key_event.modifiers.is_none());
    }

    #[cfg(feature = "which-key")]
    #[test]
    fn parse_notation_ctrl_single_char() {
        // Given a ctrl-modified single-char notation.
        let result = KeyEvent::parse_notation("c-x");

        // When parsing.
        let key_event = result.expect("should parse");

        // Then it is Ctrl+Char('x').
        assert_eq!(key_event.key, Key::Char('x'));
        assert!(key_event.modifiers.ctrl);
        assert!(!key_event.modifiers.shift);
    }

    #[cfg(feature = "which-key")]
    #[test]
    fn parse_notation_shift_single_char() {
        // Given a shift-modified single-char notation.
        let result = KeyEvent::parse_notation("s-a");

        // When parsing.
        let key_event = result.expect("should parse");

        // Then it is Shift+Char('a').
        assert_eq!(key_event.key, Key::Char('a'));
        assert!(!key_event.modifiers.ctrl);
        assert!(key_event.modifiers.shift);
    }

    #[cfg(feature = "which-key")]
    #[test]
    fn parse_notation_case_insensitive() {
        // Given a notation with mixed case.
        let result = KeyEvent::parse_notation("ENTER");

        // When parsing.
        let key_event = result.expect("should parse");

        // Then it still resolves correctly.
        assert_eq!(key_event.key, Key::Enter);
    }

    #[cfg(feature = "which-key")]
    #[rstest::rstest]
    #[case::empty("")]
    #[case::unknown("foobar")]
    #[case::bare_ctrl("c-")]
    #[case::bare_shift("s-")]
    fn parse_notation_rejects_invalid_inputs(#[case] input: &str) {
        // Given an invalid notation.
        let result = KeyEvent::parse_notation(input);

        // When parsing.

        // Then it returns None.
        assert!(result.is_none());
    }
}
