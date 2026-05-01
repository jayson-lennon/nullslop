//! Renders the conversation history.
//!
//! Each entry in the chat log is displayed with a distinct visual style so the user
//! can tell them apart at a glance:
//!
//! - **User messages** appear bold with a `>` prefix.
//! - **System messages** appear muted with indentation.
//! - **Actor messages** appear highlighted with the actor's name and content.
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
            .active_session()
            .history()
            .iter()
            .flat_map(|entry| entry_to_lines(entry))
            .collect();

        let scroll_offset = state.active_session().scroll_offset();

        // Clamp scroll_offset to prevent showing empty space below content.
        let total_wrapped: u16 = lines
            .iter()
            .map(|line| {
                let w = line.width() as u16;
                if area.width == 0 || w == 0 {
                    1
                } else {
                    w.div_ceil(area.width).max(1)
                }
            })
            .sum();
        let max_offset = total_wrapped.saturating_sub(area.height);
        let clamped = scroll_offset.min(max_offset);

        let chat_widget = Paragraph::new(lines)
            .block(Block::default().borders(Borders::NONE))
            .wrap(Wrap { trim: true })
            .scroll((clamped, 0));
        frame.render_widget(chat_widget, area);
    }
}

/// Convert a chat entry into one or more visual lines, splitting on `\n`.
///
/// The first line gets the entry-type prefix; continuation lines get indentation.
fn entry_to_lines(entry: &nullslop_protocol::ChatEntry) -> Vec<Line<'static>> {
    match &entry.kind {
        ChatEntryKind::User(text) => {
            multiline_styled(text, "> ", "  ", Style::default().add_modifier(Modifier::BOLD))
        }
        ChatEntryKind::System(text) => {
            multiline_styled(text, "  ", "  ", Style::default().fg(Color::DarkGray))
        }
        ChatEntryKind::Actor { source, text } => {
            let prefix = format!("[actor] {source}: ");
            multiline_styled(
                text,
                &prefix,
                "  ",
                Style::default().fg(Color::Yellow),
            )
        }
        ChatEntryKind::Assistant(text) => {
            multiline_styled(text, "✦ ", "  ", Style::default().fg(Color::Cyan))
        }
    }
}

/// Split text on `\n` and produce styled lines with the given prefix/indent.
fn multiline_styled(text: &str, prefix: &str, _indent: &str, style: Style) -> Vec<Line<'static>> {
    let segments = text.split('\n');
    let mut lines = Vec::new();
    for (i, segment) in segments.enumerate() {
        let content = if i == 0 {
            format!("{prefix}{segment}")
        } else {
            segment.to_string()
        };
        lines.push(Line::from(Span::styled(content, style)));
    }
    lines
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
            s.active_session_mut().push_entry(ChatEntry::user("hello"));
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
            s.active_session_mut()
                .push_entry(ChatEntry::system("ready"));
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
    fn render_actor_entry() {
        // Given a ChatLogElement with an actor entry.
        let mut element = ChatLogElement;
        let state = {
            let mut s = AppState::new();
            s.active_session_mut()
                .push_entry(ChatEntry::actor("nullslop-echo", "HELLO"));
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

        // Then the text starts with "[" (from "[actor]") and is yellow.
        let buffer = terminal.backend().buffer().clone();
        let cell = buffer.cell((0, 0)).expect("cell should exist");
        assert_eq!(cell.symbol(), "[");
        assert_eq!(cell.style().fg, Some(Color::Yellow));
    }

    #[test]
    fn render_assistant_entry() {
        // Given a ChatLogElement with an assistant entry "hello world".
        let mut element = ChatLogElement;
        let state = {
            let mut s = AppState::new();
            s.active_session_mut()
                .push_entry(ChatEntry::assistant("hello world"));
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

        // Then the first cell is "\u{2726}" (✦) and is cyan.
        let buffer = terminal.backend().buffer().clone();
        let cell = buffer.cell((0, 0)).expect("cell should exist");
        assert_eq!(cell.symbol(), "\u{2726}");
        assert_eq!(cell.style().fg, Some(Color::Cyan));
    }

    #[test]
    fn render_mixed_entries() {
        // Given a ChatLogElement with system, user, actor, and assistant entries.
        let mut element = ChatLogElement;
        let state = {
            let mut s = AppState::new();
            s.active_session_mut()
                .push_entry(ChatEntry::system("welcome"));
            s.active_session_mut().push_entry(ChatEntry::user("hello"));
            s.active_session_mut()
                .push_entry(ChatEntry::actor("nullslop-echo", "HELLO"));
            s.active_session_mut()
                .push_entry(ChatEntry::assistant("world"));
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

        // And line 2 is actor (yellow, "[" from "[actor]").
        let line2_cell = buffer.cell((0, 2)).expect("cell should exist");
        assert_eq!(line2_cell.symbol(), "[");
        assert_eq!(line2_cell.style().fg, Some(Color::Yellow));

        // And line 3 is assistant (cyan, "\u{2726}" prefix).
        let line3_cell = buffer.cell((0, 3)).expect("cell should exist");
        assert_eq!(line3_cell.symbol(), "\u{2726}");
        assert_eq!(line3_cell.style().fg, Some(Color::Cyan));
    }

    #[test]
    fn render_user_entry_with_newlines() {
        // Given a ChatLogElement with a user entry containing "hello\nworld".
        let mut element = ChatLogElement;
        let state = {
            let mut s = AppState::new();
            s.active_session_mut()
                .push_entry(ChatEntry::user("hello\nworld"));
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

        // Then line 0 has "> " prefix (bold).
        let buffer = terminal.backend().buffer().clone();
        let line0 = buffer.cell((0, 0)).expect("cell should exist");
        assert_eq!(line0.symbol(), ">");
        assert!(line0.style().add_modifier.contains(Modifier::BOLD));

        // And line 1 has "world" (no prefix, bold).
        let w_cell = buffer.cell((0, 1)).expect("cell should exist");
        assert_eq!(w_cell.symbol(), "w");
        assert!(w_cell.style().add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn render_assistant_entry_with_newlines() {
        // Given a ChatLogElement with an assistant entry containing "line1\nline2".
        let mut element = ChatLogElement;
        let state = {
            let mut s = AppState::new();
            s.active_session_mut()
                .push_entry(ChatEntry::assistant("line1\nline2"));
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

        // Then line 0 has "✦ " prefix (cyan).
        let buffer = terminal.backend().buffer().clone();
        let line0 = buffer.cell((0, 0)).expect("cell should exist");
        assert_eq!(line0.symbol(), "\u{2726}");
        assert_eq!(line0.style().fg, Some(Color::Cyan));

        // And line 1 has "line2" (no prefix, cyan).
        let l_cell = buffer.cell((0, 1)).expect("cell should exist");
        assert_eq!(l_cell.symbol(), "l");
        assert_eq!(l_cell.style().fg, Some(Color::Cyan));
    }

    #[test]
    fn render_entry_with_empty_line_between_newlines() {
        // Given a user entry "a\n\nb".
        let mut element = ChatLogElement;
        let state = {
            let mut s = AppState::new();
            s.active_session_mut().push_entry(ChatEntry::user("a\n\nb"));
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

        // Then line 0 is "> a" (bold).
        let buffer = terminal.backend().buffer().clone();
        let line0 = buffer.cell((2, 0)).expect("cell should exist");
        assert_eq!(line0.symbol(), "a");

        // And line 1 is empty (middle line between newlines).
        // And line 2 is "b" (no prefix, bold).
        let line2 = buffer.cell((0, 2)).expect("cell should exist");
        assert_eq!(line2.symbol(), "b");
        assert!(line2.style().add_modifier.contains(Modifier::BOLD));
    }
}
