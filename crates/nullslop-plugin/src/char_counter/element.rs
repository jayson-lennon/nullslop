//! UI element for the character counter.
//!
//! [`CharCounterElement`] implements [`UiElement`] to render the total
//! number of grapheme clusters in the chat input buffer.

use nullslop_plugin_ui::UiElement;
use nullslop_protocol::AppState;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::Paragraph;
use unicode_segmentation::UnicodeSegmentation;

/// Renders the character count of the chat input buffer.
///
/// Displays the number of grapheme clusters in the input buffer as
/// `chars: N`, left-aligned with default styling.
#[derive(Debug)]
pub struct CharCounterElement;

impl UiElement for CharCounterElement {
    fn name(&self) -> String {
        "char-counter".to_string()
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, state: &AppState) {
        let count = state.chat_input.input_buffer.graphemes(true).count();
        let text = format!("chars: {count}");
        let widget = Paragraph::new(text);
        frame.render_widget(widget, area);
    }
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;

    use super::*;
    use nullslop_protocol::AppState;

    #[test]
    fn name_returns_char_counter() {
        // Given a CharCounterElement.
        let element = CharCounterElement;

        // When querying the name.
        let name = element.name();

        // Then it is "char-counter".
        assert_eq!(name, "char-counter");
    }

    #[test]
    fn render_empty_buffer_shows_zero() {
        // Given a CharCounterElement with empty input buffer.
        let mut element = CharCounterElement;
        let state = AppState::new();

        let backend = TestBackend::new(40, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        let area = Rect::new(0, 0, 40, 1);

        // When rendering.
        terminal
            .draw(|frame| {
                element.render(frame, area, &state);
            })
            .unwrap();

        // Then the rendered text is "chars: 0".
        let buffer = terminal.backend().buffer().clone();
        let cell = buffer.cell((0, 0)).expect("cell should exist");
        assert_eq!(cell.symbol(), "c");
        let cell = buffer.cell((7, 0)).expect("cell should exist");
        assert_eq!(cell.symbol(), "0");
    }

    #[test]
    fn render_nonempty_buffer_shows_count() {
        // Given a CharCounterElement with "hello" in input buffer.
        let mut element = CharCounterElement;
        let state = {
            let mut s = AppState::new();
            s.chat_input.input_buffer = "hello".to_string();
            s
        };

        let backend = TestBackend::new(40, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        let area = Rect::new(0, 0, 40, 1);

        // When rendering.
        terminal
            .draw(|frame| {
                element.render(frame, area, &state);
            })
            .unwrap();

        // Then the rendered text is "chars: 5".
        let buffer = terminal.backend().buffer().clone();
        let cell = buffer.cell((7, 0)).expect("cell should exist");
        assert_eq!(cell.symbol(), "5");
    }

    #[test]
    fn render_unicode_counts_graphemes() {
        // Given a CharCounterElement with "écafé" in input buffer.
        // Each accented character is a single grapheme cluster, so count = 5.
        let mut element = CharCounterElement;
        let state = {
            let mut s = AppState::new();
            s.chat_input.input_buffer = "écafé".to_string();
            s
        };

        let backend = TestBackend::new(40, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        let area = Rect::new(0, 0, 40, 1);

        // When rendering.
        terminal
            .draw(|frame| {
                element.render(frame, area, &state);
            })
            .unwrap();

        // Then the rendered text is "chars: 5".
        let buffer = terminal.backend().buffer().clone();
        let cell = buffer.cell((7, 0)).expect("cell should exist");
        assert_eq!(cell.symbol(), "5");
    }
}
