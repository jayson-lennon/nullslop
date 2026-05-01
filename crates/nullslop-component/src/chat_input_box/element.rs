//! Renders the chat input prompt line.
//!
//! Shows the user's in-progress message below a `>` prompt. When the user is
//! actively typing (input mode), the prompt and border are highlighted in yellow and
//! the cursor appears at the current cursor position within the text. When browsing
//! (normal mode), the prompt is shown without highlighting and no cursor is displayed.

use crate::AppState;
use nullslop_component_ui::UiElement;
use nullslop_protocol::Mode;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

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

        let text_style = Style::default();

        let lines = build_lines(state.active_chat_input().text(), prompt_style, text_style);

        let block = Block::default()
            .borders(Borders::TOP)
            .border_style(border_style);
        let inner = block.inner(area);

        let input_widget = Paragraph::new(lines).block(block);
        frame.render_widget(input_widget, area);

        // Position cursor when in input mode.
        if input_mode {
            let (row, col) = state.active_chat_input().cursor_row_col();
            let prompt_width: usize = 2; // "> " = 2 columns
            let indent_width: usize = 2; // "  " = 2 columns
            let x_offset = if row == 0 { prompt_width } else { indent_width };
            let cursor_x = inner.x + (x_offset + col) as u16;
            let cursor_y = inner.y + row as u16;
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
}

/// Build visual lines from the input buffer text, splitting on `\n`.
///
/// The first line gets a `> ` prompt prefix, continuation lines get `  ` indentation.
fn build_lines<'a>(text: &str, prompt_style: Style, text_style: Style) -> Vec<Line<'a>> {
    if text.is_empty() {
        return vec![Line::from(vec![
            Span::styled("> ", prompt_style),
        ])];
    }

    let segments = text.split('\n');
    let mut lines = Vec::new();
    for (i, segment) in segments.enumerate() {
        let prefix = if i == 0 { "> " } else { "  " };
        lines.push(Line::from(vec![
            Span::styled(prefix, prompt_style),
            Span::styled(segment.to_string(), text_style),
        ]));
    }
    lines
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
            for ch in "hello".chars() {
                s.active_chat_input_mut().insert_grapheme_at_cursor(ch);
            }
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
            for ch in "hi".chars() {
                s.active_chat_input_mut().insert_grapheme_at_cursor(ch);
            }
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
            for ch in "abc".chars() {
                s.active_chat_input_mut().insert_grapheme_at_cursor(ch);
            }
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

    #[test]
    fn render_cursor_at_mid_buffer() {
        // Given a ChatInputBoxElement in Input mode with "abc" and cursor at position 1.
        let mut element = ChatInputBoxElement;
        let state = {
            let mut s = AppState::new();
            s.mode = Mode::Input;
            for ch in "abc".chars() {
                s.active_chat_input_mut().insert_grapheme_at_cursor(ch);
            }
            s.active_chat_input_mut().move_cursor_to_start();
            s.active_chat_input_mut().move_cursor_right(); // cursor at 1 (between 'a' and 'b')
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

        // Then cursor is at position (3, 1): inner.x=0 + "> "=2 + cursor_pos=1.
        terminal
            .backend_mut()
            .assert_cursor_position(Position { x: 3, y: 1 });
    }

    #[test]
    fn render_cursor_at_home() {
        // Given a ChatInputBoxElement in Input mode with "hi" and cursor moved to start.
        let mut element = ChatInputBoxElement;
        let state = {
            let mut s = AppState::new();
            s.mode = Mode::Input;
            for ch in "hi".chars() {
                s.active_chat_input_mut().insert_grapheme_at_cursor(ch);
            }
            s.active_chat_input_mut().move_cursor_to_start();
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

        // Then cursor is at position (2, 1): inner.x=0 + "> "=2 + cursor_pos=0.
        terminal
            .backend_mut()
            .assert_cursor_position(Position { x: 2, y: 1 });
    }

    #[test]
    fn render_cursor_at_end() {
        // Given a ChatInputBoxElement in Input mode with "hi" and cursor at end.
        let mut element = ChatInputBoxElement;
        let state = {
            let mut s = AppState::new();
            s.mode = Mode::Input;
            for ch in "hi".chars() {
                s.active_chat_input_mut().insert_grapheme_at_cursor(ch);
            }
            // cursor already at end (2) after inserts
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

        // Then cursor is at position (4, 1): inner.x=0 + "> "=2 + "hi"=2.
        terminal
            .backend_mut()
            .assert_cursor_position(Position { x: 4, y: 1 });
    }

    #[test]
    fn render_multiline_text_produces_multiple_lines() {
        // Given a ChatInputBoxElement with "hello\nworld" in buffer (Normal mode).
        let mut element = ChatInputBoxElement;
        let state = {
            let mut s = AppState::new();
            for ch in "hello\nworld".chars() {
                s.active_chat_input_mut().insert_grapheme_at_cursor(ch);
            }
            s
        };

        let backend = TestBackend::new(40, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let area = Rect::new(0, 0, 40, 5);

        // When rendering.
        terminal
            .draw(|frame| {
                element.render(frame, area, &state);
            })
            .unwrap();

        // Then line 1 (row 1) has "> " prefix and "hello".
        let buffer = terminal.backend().buffer().clone();
        let cell = buffer.cell((0, 1)).expect("cell should exist");
        assert_eq!(cell.symbol(), ">");
        let h_cell = buffer.cell((2, 1)).expect("cell should exist");
        assert_eq!(h_cell.symbol(), "h");

        // And line 2 (row 2) has "  " indent and "world".
        let indent_cell = buffer.cell((0, 2)).expect("cell should exist");
        assert_eq!(indent_cell.symbol(), " ");
        let w_cell = buffer.cell((2, 2)).expect("cell should exist");
        assert_eq!(w_cell.symbol(), "w");
    }

    #[test]
    fn render_multiline_cursor_on_second_line() {
        // Given a ChatInputBoxElement in Input mode with "ab\ncd" and cursor at end.
        let mut element = ChatInputBoxElement;
        let state = {
            let mut s = AppState::new();
            s.mode = Mode::Input;
            for ch in "ab\ncd".chars() {
                s.active_chat_input_mut().insert_grapheme_at_cursor(ch);
            }
            s
        };

        let backend = TestBackend::new(40, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let area = Rect::new(0, 0, 40, 5);

        // When rendering.
        terminal
            .draw(|frame| {
                element.render(frame, area, &state);
            })
            .unwrap();

        // Then cursor is at position (4, 2): row 1, col 2 ("ab\n" → row 0 col 2, "cd" → row 1 col 2).
        // inner.x=0, indent=2, col=2 → x=4, y=inner.y + 1 = 1 + 1 = 2.
        terminal
            .backend_mut()
            .assert_cursor_position(Position { x: 4, y: 2 });
    }

    #[test]
    fn render_multiline_cursor_between_newlines() {
        // Given a ChatInputBoxElement in Input mode with "a\n\nb" and cursor at the empty middle line.
        let mut element = ChatInputBoxElement;
        let state = {
            let mut s = AppState::new();
            s.mode = Mode::Input;
            for ch in "a\n\nb".chars() {
                s.active_chat_input_mut().insert_grapheme_at_cursor(ch);
            }
            // Cursor is at end (pos 4). Move back 1 to be on the empty middle line.
            s.active_chat_input_mut().move_cursor_left(); // now at pos 3, which is after the second \n, before 'b'
            // Actually: "a\n\nb" → graphemes: a(0) \n(1) \n(2) b(3). cursor at 3 = before 'b'.
            // Move left once more to be at pos 2 = after first \n, on empty line.
            s.active_chat_input_mut().move_cursor_left();
            s
        };

        let backend = TestBackend::new(40, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let area = Rect::new(0, 0, 40, 5);

        // When rendering.
        terminal
            .draw(|frame| {
                element.render(frame, area, &state);
            })
            .unwrap();

        // Then cursor is on row 1 (empty middle line), col 0.
        // inner.y=1, row=1 → y=2, indent=2, col=0 → x=2.
        terminal
            .backend_mut()
            .assert_cursor_position(Position { x: 2, y: 2 });
    }
}
