//! Streaming indicator element.
//!
//! Renders "📤 Sending..." when the active session has dispatched a message
//! but no tokens have arrived yet, "🧠 Streaming..." when tokens are arriving,
//! and renders nothing when the session is idle. Queue count is shown when
//! messages are waiting.

use crate::AppState;
use nullslop_component_ui::UiElement;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

/// Displays a streaming indicator when the active session is sending or streaming.
#[derive(Debug)]
pub struct StreamingIndicatorElement;

impl UiElement<AppState> for StreamingIndicatorElement {
    fn name(&self) -> String {
        "streaming-indicator".to_string()
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, state: &AppState) {
        let session = state.active_session();
        let queue_len = session.queue_len();

        let text = if session.is_sending() {
            if queue_len > 0 {
                format!("📤 Sending... ({queue_len} queued)")
            } else {
                "📤 Sending...".to_string()
            }
        } else if session.is_streaming() {
            if queue_len > 0 {
                format!("🧠 Streaming... ({queue_len} queued)")
            } else {
                "🧠 Streaming...".to_string()
            }
        } else {
            return;
        };

        let indicator = Paragraph::new(Line::from(Span::styled(
            text,
            Style::default().fg(Color::Cyan),
        )));
        frame.render_widget(indicator, area);
    }
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;

    use super::*;
    use crate::AppState;

    #[test]
    fn name_returns_streaming_indicator() {
        // Given a StreamingIndicatorElement.
        let element = StreamingIndicatorElement;

        // When querying the name.
        let name = element.name();

        // Then it is "streaming-indicator".
        assert_eq!(name, "streaming-indicator");
    }

    #[test]
    fn render_shows_sending_indicator() {
        // Given a StreamingIndicatorElement and a sending session.
        let mut element = StreamingIndicatorElement;
        let mut state = AppState::new();
        state.active_session_mut().begin_sending();

        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let area = Rect::new(0, 0, 40, 1);

        // When rendering.
        terminal
            .draw(|frame| {
                element.render(frame, area, &state);
            })
            .unwrap();

        // Then the buffer contains content (not empty).
        let buffer = terminal.backend().buffer().clone();
        let cell = buffer.cell((0, 0)).expect("cell should exist");
        assert_ne!(cell.symbol(), " ");
    }

    #[test]
    fn render_shows_streaming_indicator() {
        // Given a StreamingIndicatorElement and a streaming session.
        let mut element = StreamingIndicatorElement;
        let mut state = AppState::new();
        state.active_session_mut().begin_streaming();

        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let area = Rect::new(0, 0, 40, 1);

        // When rendering.
        terminal
            .draw(|frame| {
                element.render(frame, area, &state);
            })
            .unwrap();

        // Then the buffer contains content (not empty).
        let buffer = terminal.backend().buffer().clone();
        let cell = buffer.cell((0, 0)).expect("cell should exist");
        assert_ne!(cell.symbol(), " ");
    }

    #[test]
    fn render_shows_nothing_when_idle() {
        // Given a StreamingIndicatorElement and an idle session.
        let mut element = StreamingIndicatorElement;
        let state = AppState::new();

        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let area = Rect::new(0, 0, 40, 1);

        // When rendering.
        terminal
            .draw(|frame| {
                element.render(frame, area, &state);
            })
            .unwrap();

        // Then the buffer is empty.
        let buffer = terminal.backend().buffer().clone();
        let cell = buffer.cell((0, 0)).expect("cell should exist");
        assert_eq!(cell.symbol(), " ");
    }
}
