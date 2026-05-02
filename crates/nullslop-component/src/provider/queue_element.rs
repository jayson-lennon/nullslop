//! Queue display element.
//!
//! Renders stacked dimmed "QUEUED: ⟨first line⟩" entries above the input box
//! when messages are waiting in the queue.

use crate::AppState;
use nullslop_component_ui::UiElement;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

/// Displays queued messages as dimmed entries.
#[derive(Debug)]
pub struct QueueDisplayElement;

impl UiElement<AppState> for QueueDisplayElement {
    fn name(&self) -> String {
        "queue-display".to_owned()
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, state: &AppState) {
        let queue = state.active_session().queue();
        if queue.is_empty() {
            return;
        }

        let lines: Vec<Line> = queue
            .iter()
            .map(|msg| {
                let first_line = msg.lines().next().unwrap_or("");
                let display = if first_line.len() > 60 {
                    let truncated: String = first_line.chars().take(59).collect();
                    format!("QUEUED: {truncated}…")
                } else {
                    format!("QUEUED: {first_line}")
                };
                Line::from(Span::styled(display, Style::default().fg(Color::DarkGray)))
            })
            .collect();

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, area);
    }
}
