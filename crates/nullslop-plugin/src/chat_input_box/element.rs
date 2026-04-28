//! UI element for the chat input box.
//!
//! [`ChatInputBoxElement`] implements [`UiElement`] to render the input box.
//! It reads from `AppData.input_buffer` and renders the input prompt.

use nullslop_plugin_ui::UiElement;
use ratatui::widgets::{Block, Borders, Paragraph};

/// Renders the chat input box.
///
/// Displays the current input buffer with a `>` prompt and a top border.
#[derive(Debug)]
pub struct ChatInputBoxElement;

impl UiElement for ChatInputBoxElement {
    fn name(&self) -> String {
        "chat-input-box".to_string()
    }

    fn render(
        &mut self,
        frame: &mut ratatui::Frame<'_>,
        area: ratatui::layout::Rect,
        state: &nullslop_protocol::AppData,
    ) {
        let input_text = format!("> {}", state.input_buffer);
        let input_widget = Paragraph::new(input_text)
            .block(Block::default().borders(Borders::TOP))
            .style(ratatui::style::Style::default());
        frame.render_widget(input_widget, area);
    }
}

#[cfg(test)]
mod tests {
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;
    use ratatui::Terminal;

    use super::*;

    #[test]
    fn name_returns_chat_input_box() {
        // Given a ChatInputBoxElement.
        let element = ChatInputBoxElement;

        // When querying the name.
        let name = element.name();

        // Then it is "chat-input-box".
        assert_eq!(name, "chat-input-box");
    }

    #[test]
    fn render_draws_input_buffer() {
        // Given a ChatInputBoxElement with "hello" in state.
        let mut element = ChatInputBoxElement;
        let state = {
            let mut s = nullslop_protocol::AppData::new();
            s.input_buffer = "hello".to_string();
            s
        };

        let backend = TestBackend::new(40, 3);
        let mut terminal = Terminal::new(backend).unwrap();
        let area = Rect::new(0, 0, 40, 3);

        // When rendering.
        terminal
            .draw(|frame| {
                element.render(frame, area, &state);
            })
            .unwrap();

        // Then the buffer contains the ">" prompt character.
        let buffer = terminal.backend().buffer().clone();
        let cell = buffer.cell((0, 1)).expect("cell should exist");
        assert_eq!(cell.symbol(), ">");
    }

    #[test]
    fn render_draws_empty_buffer() {
        // Given a ChatInputBoxElement with empty state.
        let mut element = ChatInputBoxElement;
        let state = nullslop_protocol::AppData::new();

        let backend = TestBackend::new(20, 3);
        let mut terminal = Terminal::new(backend).unwrap();
        let area = Rect::new(0, 0, 20, 3);

        // When rendering.
        terminal
            .draw(|frame| {
                element.render(frame, area, &state);
            })
            .unwrap();

        // Then the buffer is rendered without panic and shows prompt.
        let buffer = terminal.backend().buffer().clone();
        let cell = buffer.cell((0, 1)).expect("cell should exist");
        assert_eq!(cell.symbol(), ">");
    }
}
