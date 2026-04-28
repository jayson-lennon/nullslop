//! UI element for the chat log.
//!
//! [`ChatLogElement`] implements [`UiElement`] to render chat history.
//! It reads from `AppData.chat_history` and displays user entries in bold
//! with a `> ` prefix and system entries in dark gray with a `  ` prefix.

use nullslop_plugin_ui::UiElement;
use nullslop_protocol::ChatEntryKind;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

/// Renders the chat history log.
///
/// Displays all chat entries from `AppData.chat_history`, styling user
/// entries in bold with a `> ` prefix and system entries in dark gray
/// with a `  ` prefix. Text wraps within the allocated area.
#[derive(Debug)]
pub struct ChatLogElement;

impl UiElement for ChatLogElement {
    fn name(&self) -> String {
        "chat-log".to_string()
    }

    fn render(
        &mut self,
        frame: &mut ratatui::Frame<'_>,
        area: ratatui::layout::Rect,
        state: &nullslop_protocol::AppData,
    ) {
        let lines: Vec<Line> = state
            .chat_history
            .iter()
            .map(|entry| match &entry.kind {
                ChatEntryKind::User(text) => Line::from(Span::styled(
                    format!("> {text}"),
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                ChatEntryKind::System(text) => Line::from(Span::styled(
                    format!("  {text}"),
                    Style::default().fg(Color::DarkGray),
                )),
            })
            .collect();

        let chat_widget = Paragraph::new(lines)
            .block(Block::default().borders(Borders::NONE))
            .wrap(Wrap { trim: true });
        frame.render_widget(chat_widget, area);
    }
}

#[cfg(test)]
mod tests {
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;
    use ratatui::Terminal;

    use super::*;
    use nullslop_protocol::{AppData, ChatEntry};

    #[test]
    fn name_returns_chat_log() {
        // Given a ChatLogElement.
        let element = ChatLogElement;

        // When querying the name.
        let name = element.name();

        // Then it is "chat-log".
        assert_eq!(name, "chat-log");
    }

    #[test]
    fn render_empty_history() {
        // Given a ChatLogElement with empty chat history.
        let mut element = ChatLogElement;
        let state = AppData::new();

        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let area = Rect::new(0, 0, 40, 10);

        // When rendering.
        terminal
            .draw(|frame| {
                element.render(frame, area, &state);
            })
            .unwrap();

        // Then it renders without panic and the first cell is empty.
        let buffer = terminal.backend().buffer().clone();
        let cell = buffer.cell((0, 0)).expect("cell should exist");
        assert_eq!(cell.symbol(), " ");
    }

    #[test]
    fn render_user_entry() {
        // Given a ChatLogElement with a user entry "hello".
        let mut element = ChatLogElement;
        let state = {
            let mut s = AppData::new();
            s.chat_history.push(ChatEntry::user("hello"));
            s
        };

        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let area = Rect::new(0, 0, 40, 10);

        // When rendering.
        terminal
            .draw(|frame| {
                element.render(frame, area, &state);
            })
            .unwrap();

        // Then the first cell is ">" and the text is bold.
        let buffer = terminal.backend().buffer().clone();
        let cell = buffer.cell((0, 0)).expect("cell should exist");
        assert_eq!(cell.symbol(), ">");
        assert!(cell.style().add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn render_system_entry() {
        // Given a ChatLogElement with a system entry "ready".
        let mut element = ChatLogElement;
        let state = {
            let mut s = AppData::new();
            s.chat_history.push(ChatEntry::system("ready"));
            s
        };

        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let area = Rect::new(0, 0, 40, 10);

        // When rendering.
        terminal
            .draw(|frame| {
                element.render(frame, area, &state);
            })
            .unwrap();

        // Then the text is dark gray. (Leading spaces are trimmed by Wrap { trim: true }.)
        let buffer = terminal.backend().buffer().clone();
        let cell = buffer.cell((0, 0)).expect("cell should exist");
        assert_eq!(cell.symbol(), "r");
        assert_eq!(cell.style().fg, Some(Color::DarkGray));
    }

    #[test]
    fn render_mixed_entries() {
        // Given a ChatLogElement with both user and system entries.
        let mut element = ChatLogElement;
        let state = {
            let mut s = AppData::new();
            s.chat_history.push(ChatEntry::system("welcome"));
            s.chat_history.push(ChatEntry::user("hello"));
            s.chat_history.push(ChatEntry::system("received"));
            s
        };

        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let area = Rect::new(0, 0, 40, 10);

        // When rendering.
        terminal
            .draw(|frame| {
                element.render(frame, area, &state);
            })
            .unwrap();

        // Then line 0 is system (dark gray). (Leading spaces trimmed by Wrap.)
        let buffer = terminal.backend().buffer().clone();
        let line0_cell = buffer.cell((0, 0)).expect("cell should exist");
        assert_eq!(line0_cell.symbol(), "w");
        assert_eq!(line0_cell.style().fg, Some(Color::DarkGray));

        // And line 1 is user (">" prefix, bold).
        let line1_cell = buffer.cell((0, 1)).expect("cell should exist");
        assert_eq!(line1_cell.symbol(), ">");
        assert!(line1_cell.style().add_modifier.contains(Modifier::BOLD));

        // And line 2 is system again (dark gray). (Leading spaces trimmed by Wrap.)
        let line2_cell = buffer.cell((0, 2)).expect("cell should exist");
        assert_eq!(line2_cell.symbol(), "r");
        assert_eq!(line2_cell.style().fg, Some(Color::DarkGray));
    }
}
