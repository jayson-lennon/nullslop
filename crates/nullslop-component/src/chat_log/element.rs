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
        "chat-log".to_owned()
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, state: &AppState) {
        let lines: Vec<Line> = state
            .active_session()
            .history()
            .iter()
            .flat_map(|entry| entry_to_lines(entry))
            .collect();

        // Calculate total wrapped lines.
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

        // Bottom-align: when content fits within the viewport, prepend blank lines
        // so messages appear at the bottom with empty space above.
        let blank_count = area.height.saturating_sub(total_wrapped) as usize;
        let mut display_lines = Vec::with_capacity(blank_count + lines.len());
        for _ in 0..blank_count {
            display_lines.push(Line::from(""));
        }
        display_lines.extend(lines);

        let scroll_offset = state.active_session().scroll_offset();

        // Clamp scroll_offset: when padded to fill, max_offset is 0 (no scrolling).
        // When content overflows, allow scrolling up to total − viewport height.
        let total_display = total_wrapped + blank_count as u16;
        let max_offset = total_display.saturating_sub(area.height);
        let clamped = scroll_offset.min(max_offset);

        let chat_widget = Paragraph::new(display_lines)
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
        ChatEntryKind::User(text) => multiline_styled(
            text,
            "> ",
            "  ",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        ChatEntryKind::System(text) => {
            multiline_styled(text, "  ", "  ", Style::default().fg(Color::DarkGray))
        }
        ChatEntryKind::Actor { source, text } => {
            let prefix = format!("[actor] {source}: ");
            multiline_styled(text, &prefix, "  ", Style::default().fg(Color::Yellow))
        }
        ChatEntryKind::Assistant(text) => {
            multiline_styled(text, "✦ ", "  ", Style::default().fg(Color::Cyan))
        }
        ChatEntryKind::ToolCall {
            id: _,
            name,
            arguments,
        } => multiline_styled(
            format!("🔧 {name}({arguments})"),
            "  ",
            "  ",
            Style::default().fg(Color::Magenta),
        ),
        ChatEntryKind::ToolResult {
            id: _,
            name,
            content,
            success,
        } => {
            let icon = if *success { "✅" } else { "❌" };
            multiline_styled(
                format!("{icon} {name}: {content}"),
                "  ",
                "  ",
                if *success {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Red)
                },
            )
        }
    }
}

/// Split text on `\n` and produce styled lines with the given prefix/indent.
fn multiline_styled<T, P, I>(text: T, prefix: P, indent: I, style: Style) -> Vec<Line<'static>>
where
    T: AsRef<str>,
    P: AsRef<str>,
    I: AsRef<str>,
{
    let text = text.as_ref();
    let prefix = prefix.as_ref();
    let _ = indent.as_ref();
    let segments = text.split('\n');
    let mut lines = Vec::new();
    for (i, segment) in segments.enumerate() {
        let content = if i == 0 {
            format!("{prefix}{segment}")
        } else {
            segment.to_owned()
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
        let state = AppState::default();

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
            let mut s = AppState::default();
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

        // Then the bottom row has ">" and the text is bold.
        let buffer = terminal.backend().buffer().clone();
        let cell = buffer.cell((0, 9)).expect("cell should exist");
        assert_eq!(cell.symbol(), ">");
        assert!(cell.style().add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn render_system_entry() {
        // Given a ChatLogElement with a system entry "ready".
        let mut element = ChatLogElement;
        let state = {
            let mut s = AppState::default();
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

        // Then the text is dark gray on the bottom row.
        let buffer = terminal.backend().buffer().clone();
        let cell = buffer.cell((0, 9)).expect("cell should exist");
        assert_eq!(cell.symbol(), "r");
        assert_eq!(cell.style().fg, Some(Color::DarkGray));
    }

    #[test]
    fn render_actor_entry() {
        // Given a ChatLogElement with an actor entry.
        let mut element = ChatLogElement;
        let state = {
            let mut s = AppState::default();
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

        // Then the text starts with "[" (from "[actor]") on the bottom row and is yellow.
        let buffer = terminal.backend().buffer().clone();
        let cell = buffer.cell((0, 9)).expect("cell should exist");
        assert_eq!(cell.symbol(), "[");
        assert_eq!(cell.style().fg, Some(Color::Yellow));
    }

    #[test]
    fn render_assistant_entry() {
        // Given a ChatLogElement with an assistant entry "hello world".
        let mut element = ChatLogElement;
        let state = {
            let mut s = AppState::default();
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

        // Then the bottom row has "\u{2726}" (✦) and is cyan.
        let buffer = terminal.backend().buffer().clone();
        let cell = buffer.cell((0, 9)).expect("cell should exist");
        assert_eq!(cell.symbol(), "\u{2726}");
        assert_eq!(cell.style().fg, Some(Color::Cyan));
    }

    #[test]
    fn render_mixed_entries() {
        // Given a ChatLogElement with system, user, actor, and assistant entries.
        let mut element = ChatLogElement;
        let state = {
            let mut s = AppState::default();
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

        // Then line 6 is system (dark gray).
        let buffer = terminal.backend().buffer().clone();
        let line6_cell = buffer.cell((0, 6)).expect("cell should exist");
        assert_eq!(line6_cell.symbol(), "w");
        assert_eq!(line6_cell.style().fg, Some(Color::DarkGray));

        // And line 7 is user (">" prefix, bold).
        let line7_cell = buffer.cell((0, 7)).expect("cell should exist");
        assert_eq!(line7_cell.symbol(), ">");
        assert!(line7_cell.style().add_modifier.contains(Modifier::BOLD));

        // And line 8 is actor (yellow, "[" from "[actor]").
        let line8_cell = buffer.cell((0, 8)).expect("cell should exist");
        assert_eq!(line8_cell.symbol(), "[");
        assert_eq!(line8_cell.style().fg, Some(Color::Yellow));

        // And line 9 is assistant (cyan, "\u{2726}" prefix).
        let line9_cell = buffer.cell((0, 9)).expect("cell should exist");
        assert_eq!(line9_cell.symbol(), "\u{2726}");
        assert_eq!(line9_cell.style().fg, Some(Color::Cyan));
    }

    #[test]
    fn render_user_entry_with_newlines() {
        // Given a ChatLogElement with a user entry containing "hello\nworld".
        let mut element = ChatLogElement;
        let state = {
            let mut s = AppState::default();
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

        // Then line 8 has "> " prefix (bold).
        let buffer = terminal.backend().buffer().clone();
        let line8 = buffer.cell((0, 8)).expect("cell should exist");
        assert_eq!(line8.symbol(), ">");
        assert!(line8.style().add_modifier.contains(Modifier::BOLD));

        // And line 9 has "world" (no prefix, bold).
        let w_cell = buffer.cell((0, 9)).expect("cell should exist");
        assert_eq!(w_cell.symbol(), "w");
        assert!(w_cell.style().add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn render_assistant_entry_with_newlines() {
        // Given a ChatLogElement with an assistant entry containing "line1\nline2".
        let mut element = ChatLogElement;
        let state = {
            let mut s = AppState::default();
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

        // Then line 8 has "✦ " prefix (cyan).
        let buffer = terminal.backend().buffer().clone();
        let line8 = buffer.cell((0, 8)).expect("cell should exist");
        assert_eq!(line8.symbol(), "\u{2726}");
        assert_eq!(line8.style().fg, Some(Color::Cyan));

        // And line 9 has "line2" (no prefix, cyan).
        let l_cell = buffer.cell((0, 9)).expect("cell should exist");
        assert_eq!(l_cell.symbol(), "l");
        assert_eq!(l_cell.style().fg, Some(Color::Cyan));
    }

    #[test]
    fn render_entry_with_empty_line_between_newlines() {
        // Given a user entry "a\n\nb".
        let mut element = ChatLogElement;
        let state = {
            let mut s = AppState::default();
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

        // Then line 7 is "> a" (bold).
        let buffer = terminal.backend().buffer().clone();
        let line7 = buffer.cell((2, 7)).expect("cell should exist");
        assert_eq!(line7.symbol(), "a");

        // And line 8 is empty (middle line between newlines).
        // And line 9 is "b" (no prefix, bold).
        let line9 = buffer.cell((0, 9)).expect("cell should exist");
        assert_eq!(line9.symbol(), "b");
        assert!(line9.style().add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn render_few_messages_bottom_aligned() {
        // Given a ChatLogElement with one user entry in a 40x10 viewport.
        let mut element = ChatLogElement;
        let state = {
            let mut s = AppState::default();
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

        // Then the top rows are empty and the message appears at the bottom.
        let buffer = terminal.backend().buffer().clone();

        // Top row is empty.
        let top_cell = buffer.cell((0, 0)).expect("cell should exist");
        assert_eq!(top_cell.symbol(), " ");

        // Bottom row has the user message.
        let bottom_cell = buffer.cell((0, 9)).expect("cell should exist");
        assert_eq!(bottom_cell.symbol(), ">");
        assert!(bottom_cell.style().add_modifier.contains(Modifier::BOLD));
    }
}
