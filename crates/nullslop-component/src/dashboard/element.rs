//! Renders the dashboard view — a list of actors with their startup status.
//!
//! Each actor is displayed as a row with its name and status badge.
//! "Starting" appears yellow, "Started" appears green.

use crate::AppState;
use crate::dashboard::state::ActorStatus;
use nullslop_component_ui::UiElement;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

/// Display element for the actor dashboard.
#[derive(Debug)]
pub struct DashboardElement;

impl UiElement<AppState> for DashboardElement {
    fn name(&self) -> String {
        "dashboard".to_owned()
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, state: &AppState) {
        let lines: Vec<Line> = if state.dashboard.actors().is_empty() {
            vec![Line::from(Span::styled(
                "No actors registered.",
                Style::default().fg(Color::DarkGray),
            ))]
        } else {
            let mut lines = Vec::new();

            // Compute the widest name for column alignment (header or data).
            let header_name = "Actor";
            let max_name_len = state
                .dashboard
                .actors()
                .into_iter()
                .map(|(name, _)| name.len())
                .chain(std::iter::once(header_name.len()))
                .max()
                .unwrap_or(header_name.len());

            // Header row.
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {header_name:<max_name_len$} "),
                    Style::default().fg(Color::Gray).bold(),
                ),
                Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
                Span::styled("Status", Style::default().fg(Color::Gray).bold()),
            ]));

            for (name, status) in state.dashboard.actors() {
                let (label, color) = match status {
                    ActorStatus::Starting => ("Starting", Color::Yellow),
                    ActorStatus::Started => ("Started", Color::Green),
                };
                let padded_name = format!(" {name:<max_name_len$} ");
                lines.push(Line::from(vec![
                    Span::styled(padded_name, Style::default()),
                    Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
                    Span::styled(label, Style::default().fg(color)),
                ]));
            }

            lines
        };

        let widget = Paragraph::new(lines)
            .block(Block::default().borders(Borders::NONE))
            .wrap(Wrap { trim: true });
        frame.render_widget(widget, area);
    }
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;

    use super::*;
    use crate::test_utils;
    use crate::AppState;

    fn render_rows(element: &mut DashboardElement, state: &AppState) -> Vec<String> {
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let area = Rect::new(0, 0, 40, 10);
        terminal
            .draw(|frame| {
                element.render(frame, area, state);
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        (0..10)
            .map(|y| {
                (0..40)
                    .map(|x| {
                        buffer
                            .cell((x, y))
                            .map_or(" ", ratatui::buffer::Cell::symbol)
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn name_returns_dashboard() {
        // Given a DashboardElement.
        let element = DashboardElement;

        // When querying the name.
        let name = element.name();

        // Then it is "dashboard".
        assert_eq!(name, "dashboard");
    }

    #[test]
    fn render_empty_shows_no_actors() {
        // Given a DashboardElement with no actors.
        let mut element = DashboardElement;
        let state = AppState::new(test_utils::test_services());

        // When rendering.
        let rows = render_rows(&mut element, &state);

        // Then "No actors registered." appears.
        assert!(rows[0].contains("No actors registered."));
    }

    #[test]
    fn render_actor_with_starting_status() {
        // Given a DashboardElement with an actor in Starting status.
        let mut element = DashboardElement;
        let state = {
            let mut s = AppState::new(test_utils::test_services());
            s.dashboard.mark_starting("actor-a");
            s
        };

        // When rendering.
        let rows = render_rows(&mut element, &state);

        // Then the actor name and status appear.
        assert!(rows[1].contains("actor-a"));
        assert!(rows[1].contains("Starting"));
    }

    #[test]
    fn render_actor_with_started_status() {
        // Given a DashboardElement with an actor in Started status.
        let mut element = DashboardElement;
        let state = {
            let mut s = AppState::new(test_utils::test_services());
            s.dashboard.mark_starting("actor-a");
            s.dashboard.mark_started("actor-a");
            s
        };

        // When rendering.
        let rows = render_rows(&mut element, &state);

        // Then the actor name and status appear.
        assert!(rows[1].contains("actor-a"));
        assert!(rows[1].contains("Started"));
    }
}
