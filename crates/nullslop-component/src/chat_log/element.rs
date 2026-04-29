//! Renders the conversation history.
//!
//! Each entry in the chat log is displayed with a distinct visual style so the user
//! can tell them apart at a glance:
//!
//! - **User messages** appear bold with a `>` prefix.
//! - **System messages** appear muted with indentation.
//! - **Extension messages** appear highlighted with the extension's name and content.
//!
//! Text wraps within the available space.

use crate::AppState;
use nullslop_component_ui::UiElement;
use nullslop_protocol::ChatEntryKind;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

/// Display element for the full conversation history.
#[derive(Debug)]
pub struct ChatLogElement;

impl UiElement<AppState> for ChatLogElement {
    fn name(&self) -> String {
        "chat-log".to_string()
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, state: &AppState) {
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
                ChatEntryKind::Extension { source, text } => Line::from(Span::styled(
                    format!("[ext] {source}: {text}"),
                    Style::default().fg(Color::Yellow),
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
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;

    use super::*;
    use crate::AppState;
    use nullslop_protocol::ChatEntry;

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
        let state = AppState::new();

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
            let mut s = AppState::new();
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
            let mut s = AppState::new();
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
    fn render_extension_entry() {
        // Given a ChatLogElement with an extension entry.
        let mut element = ChatLogElement;
        let state = {
            let mut s = AppState::new();
            s.chat_history
                .push(ChatEntry::extension("nullslop-echo", "HELLO"));
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

        // Then the text starts with "[" (from "[ext]") and is yellow.
        let buffer = terminal.backend().buffer().clone();
        let cell = buffer.cell((0, 0)).expect("cell should exist");
        assert_eq!(cell.symbol(), "[");
        assert_eq!(cell.style().fg, Some(Color::Yellow));
    }

    #[test]
    fn render_mixed_entries() {
        // Given a ChatLogElement with user, system, and extension entries.
        let mut element = ChatLogElement;
        let state = {
            let mut s = AppState::new();
            s.chat_history.push(ChatEntry::system("welcome"));
            s.chat_history.push(ChatEntry::user("hello"));
            s.chat_history
                .push(ChatEntry::extension("nullslop-echo", "HELLO"));
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

        // And line 2 is extension (yellow, "[" from "[ext]").
        let line2_cell = buffer.cell((0, 2)).expect("cell should exist");
        assert_eq!(line2_cell.symbol(), "[");
        assert_eq!(line2_cell.style().fg, Some(Color::Yellow));
    }
}
