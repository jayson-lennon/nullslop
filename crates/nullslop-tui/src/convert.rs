//! Conversions from crossterm key types to nullslop-core key types.
//!
//! These conversions are retained for use by extensions that need
//! `nullslop_core::KeyEvent` conversion. The main key handling path
//! now routes through `ratatui-which-key` directly.

#![allow(dead_code)]

use nullslop_core::{Key, KeyEvent, Modifiers};

/// Converts a crossterm `KeyEvent` to a nullslop-core `KeyEvent`.
///
/// Returns `None` for crossterm key codes that have no nullslop equivalent
/// (e.g., `KeyCode::Null`, `KeyCode::Modifier`).
#[must_use]
pub fn from_crossterm(event: crossterm::event::KeyEvent) -> Option<KeyEvent> {
    let key = match event.code {
        crossterm::event::KeyCode::Char(c) => Key::Char(c),
        crossterm::event::KeyCode::Enter => Key::Enter,
        crossterm::event::KeyCode::Esc => Key::Esc,
        crossterm::event::KeyCode::Tab => Key::Tab,
        crossterm::event::KeyCode::Backspace => Key::Backspace,
        crossterm::event::KeyCode::Up => Key::Up,
        crossterm::event::KeyCode::Down => Key::Down,
        crossterm::event::KeyCode::Left => Key::Left,
        crossterm::event::KeyCode::Right => Key::Right,
        crossterm::event::KeyCode::Home => Key::Home,
        crossterm::event::KeyCode::End => Key::End,
        crossterm::event::KeyCode::PageUp => Key::PageUp,
        crossterm::event::KeyCode::PageDown => Key::PageDown,
        crossterm::event::KeyCode::F(n) => Key::F(n),
        _ => return None,
    };

    let modifiers = Modifiers {
        ctrl: event
            .modifiers
            .contains(crossterm::event::KeyModifiers::CONTROL),
        alt: event
            .modifiers
            .contains(crossterm::event::KeyModifiers::ALT),
        shift: event
            .modifiers
            .contains(crossterm::event::KeyModifiers::SHIFT),
    };

    Some(KeyEvent { key, modifiers })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn crossterm_key(code: crossterm::event::KeyCode) -> crossterm::event::KeyEvent {
        crossterm::event::KeyEvent::new(code, crossterm::event::KeyModifiers::NONE)
    }

    fn crossterm_key_with_mod(
        code: crossterm::event::KeyCode,
        modifiers: crossterm::event::KeyModifiers,
    ) -> crossterm::event::KeyEvent {
        crossterm::event::KeyEvent::new(code, modifiers)
    }

    #[test]
    fn convert_char_key() {
        // Given crossterm Char('a').
        let event = crossterm_key(crossterm::event::KeyCode::Char('a'));

        // When converting.
        let result = from_crossterm(event);

        // Then returns Key::Char('a') with no modifiers.
        let key_event = result.expect("should convert");
        assert_eq!(key_event.key, Key::Char('a'));
        assert!(key_event.modifiers.is_none());
    }

    #[test]
    fn convert_ctrl_enter() {
        // Given crossterm Enter with CONTROL.
        let event = crossterm_key_with_mod(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::CONTROL,
        );

        // When converting.
        let result = from_crossterm(event);

        // Then returns Key::Enter with ctrl=true.
        let key_event = result.expect("should convert");
        assert_eq!(key_event.key, Key::Enter);
        assert!(key_event.modifiers.ctrl);
    }

    #[test]
    fn convert_f_key() {
        // Given crossterm F(5).
        let event = crossterm_key(crossterm::event::KeyCode::F(5));

        // When converting.
        let result = from_crossterm(event);

        // Then returns Key::F(5).
        let key_event = result.expect("should convert");
        assert_eq!(key_event.key, Key::F(5));
    }

    #[test]
    fn convert_unknown_returns_none() {
        // Given crossterm Null.
        let event = crossterm_key(crossterm::event::KeyCode::Null);

        // When converting.
        let result = from_crossterm(event);

        // Then returns None.
        assert_eq!(result, None);
    }
}
