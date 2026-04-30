//! Renders the chat input prompt line.
//!
//! Shows the user's in-progress message below a `>` prompt. When the user is
//! actively typing (input mode), the prompt and border are highlighted in yellow and
//! the cursor appears at the end of the text. When browsing (normal mode), the
//! prompt is shown without highlighting and no cursor is displayed.

use crate::AppState;
use nullslop_component_ui::UiElement;
use nullslop_protocol::Mode;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use unicode_segmentation::UnicodeSegmentation;

/// Display element for the user's message composition area.
#[derive(Debug)]
pub struct ChatInputBoxElement;

impl UiElement<AppState> for ChatInputBoxElement {
    fn name(&self) -> String {
        "chat-input-box".to_string()
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, state: &AppState) {
        let input_mode = state.mode == Mode::Input;

        let prompt_style = if input_mode {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().add_modifier(Modifier::BOLD)
        };

        let border_style = if input_mode {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        let line = Line::from(vec![
            Span::styled("> ", prompt_style),
            Span::styled(&state.chat_input.input_buffer, Style::default()),
        ]);

        let block = Block::default()
            .borders(Borders::TOP)
            .border_style(border_style);
        let inner = block.inner(area);

        let input_widget = Paragraph::new(line).block(block);
        frame.render_widget(input_widget, area);

        // Position cursor at the end of the prompt + text when in input mode.
        if input_mode {
            let prompt_width: usize = 2; // "> " = 2 columns
            let text_width: usize = state.chat_input.input_buffer.graphemes(true).count();
            let cursor_x = inner.x + (prompt_width + text_width) as u16;
            let cursor_y = inner.y;
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::{Backend, TestBackend};
    use ratatui::layout::{Position, Rect};

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
        // Given a ChatInputBoxElement with "hello" in state (Normal mode).
        let mut element = ChatInputBoxElement;
        let state = {
            let mut s = AppState::new();
            s.chat_input.input_buffer = "hello".to_string();
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
        let state = AppState::new();

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

    #[test]
    fn render_input_mode_yellow_prompt() {
        // Given a ChatInputBoxElement in Input mode with "hi" in buffer.
        let mut element = ChatInputBoxElement;
        let state = {
            let mut s = AppState::new();
            s.mode = Mode::Input;
            s.chat_input.input_buffer = "hi".to_string();
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

        // Then the ">" prompt is yellow.
        let buffer = terminal.backend().buffer().clone();
        let cell = buffer.cell((0, 1)).expect("cell should exist");
        assert_eq!(cell.symbol(), ">");
        assert_eq!(cell.style().fg, Some(Color::Yellow));
    }

    #[test]
    fn render_input_mode_yellow_border() {
        // Given a ChatInputBoxElement in Input mode.
        let mut element = ChatInputBoxElement;
        let state = {
            let mut s = AppState::new();
            s.mode = Mode::Input;
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

        // Then the top border is yellow.
        let buffer = terminal.backend().buffer().clone();
        let cell = buffer.cell((0, 0)).expect("cell should exist");
        assert_eq!(cell.style().fg, Some(Color::Yellow));
    }

    #[test]
    fn render_input_mode_cursor_at_end_of_text() {
        // Given a ChatInputBoxElement in Input mode with "abc" in buffer.
        let mut element = ChatInputBoxElement;
        let state = {
            let mut s = AppState::new();
            s.mode = Mode::Input;
            s.chat_input.input_buffer = "abc".to_string();
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

        // Then cursor is at position (5, 1): inner.x=0 + "> "=2 + "abc"=3.
        terminal
            .backend_mut()
            .assert_cursor_position(Position { x: 5, y: 1 });
    }

    #[test]
    fn render_normal_mode_no_cursor() {
        // Given a ChatInputBoxElement in Normal mode.
        let mut element = ChatInputBoxElement;
        let state = AppState::new();

        let backend = TestBackend::new(40, 3);
        let mut terminal = Terminal::new(backend).unwrap();
        let area = Rect::new(0, 0, 40, 3);

        // When rendering.
        terminal
            .draw(|frame| {
                element.render(frame, area, &state);
            })
            .unwrap();

        // Then cursor position was not set (remains at default 0,0 with cursor hidden).
        let pos = terminal.backend_mut().get_cursor_position().unwrap();
        assert_eq!(pos, Position { x: 0, y: 0 });
    }
}
