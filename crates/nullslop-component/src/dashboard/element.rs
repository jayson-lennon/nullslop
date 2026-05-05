//! Renders the dashboard view — a list of actors with their startup status.
//!
//! Each actor is displayed as a card with its name and status on one line,
//! and a light gray description indented below. "Starting" appears yellow,
//! "Running" appears green.

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

            for (i, entry) in state.dashboard.actors().iter().enumerate() {
                let (label, color) = match entry.status {
                    ActorStatus::Starting => ("Starting", Color::Yellow),
                    ActorStatus::Running => ("Running", Color::Green),
                };

                // Name line: padded name ... status
                lines.push(Line::from(vec![
                    Span::styled(
                        format!(" {}", entry.name),
                        Style::default().bold(),
                    ),
                    // Fill with spaces to push status right — use raw spaces
                    Span::raw(fill_to_status(&entry.name, area.width)),
                    Span::styled(label, Style::default().fg(color)),
                ]));

                // Description line (if present).
                if let Some(desc) = &entry.description {
                    lines.push(Line::from(Span::styled(
                        format!("   {}", desc),
                        Style::default().fg(Color::DarkGray),
                    )));
                }

                // Blank line between actors (not after the last one).
                if i < state.dashboard.actors().len() - 1 {
                    lines.push(Line::from(""));
                }
            }

            lines
        };

        let widget = Paragraph::new(lines)
            .block(Block::default().borders(Borders::NONE))
            .wrap(Wrap { trim: true });
        frame.render_widget(widget, area);
    }
}

/// Returns spaces to pad between the name and the right-aligned status.
/// The status label takes up to ~8 chars ("Starting"), so we leave room.
fn fill_to_status(name: &str, area_width: u16) -> String {
    let status_width: usize = 8; // "Starting" is the longest status
    let name_len = name.len() + 1; // +1 for leading space
    let available = area_width as usize;
    let padding = available.saturating_sub(name_len).saturating_sub(status_width);
    " ".repeat(padding.max(1))
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;

    use super::*;
    use crate::AppState;

    fn render_rows(element: &mut DashboardElement, state: &AppState, width: u16, height: u16) -> Vec<String> {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let area = Rect::new(0, 0, width, height);
        terminal
            .draw(|frame| {
                element.render(frame, area, state);
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        (0..height)
            .map(|y| {
                (0..width)
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
        let state = AppState::default();

        // When rendering.
        let rows = render_rows(&mut element, &state, 40, 10);

        // Then "No actors registered." appears.
        assert!(rows[0].contains("No actors registered."));
    }

    #[test]
    fn render_actor_with_starting_status() {
        // Given a DashboardElement with an actor in Starting status.
        let mut element = DashboardElement;
        let state = {
            let mut s = AppState::default();
            s.dashboard.mark_starting("echo", Some("Echoes messages back".to_string()));
            s
        };

        // When rendering.
        let rows = render_rows(&mut element, &state, 40, 10);

        // Then the actor name and status appear on the first line.
        assert!(rows[0].contains("echo"));
        assert!(rows[0].contains("Starting"));

        // And the description appears on the next line in light gray.
        assert!(rows[1].contains("Echoes messages back"));
    }

    #[test]
    fn render_actor_with_running_status() {
        // Given a DashboardElement with an actor in Running status.
        let mut element = DashboardElement;
        let state = {
            let mut s = AppState::default();
            s.dashboard.mark_starting("echo", Some("Echoes messages back".to_string()));
            s.dashboard.mark_running("echo", None);
            s
        };

        // When rendering.
        let rows = render_rows(&mut element, &state, 40, 10);

        // Then the actor name and Running status appear.
        assert!(rows[0].contains("echo"));
        assert!(rows[0].contains("Running"));
    }

    #[test]
    fn render_actor_without_description() {
        // Given a DashboardElement with an actor that has no description.
        let mut element = DashboardElement;
        let state = {
            let mut s = AppState::default();
            s.dashboard.mark_starting("actor-a", None);
            s
        };

        // When rendering.
        let rows = render_rows(&mut element, &state, 40, 10);

        // Then the actor name and status appear with no description line.
        assert!(rows[0].contains("actor-a"));
        assert!(rows[0].contains("Starting"));
    }

    #[test]
    fn render_multiple_actors_with_blank_line_between() {
        // Given two actors.
        let mut element = DashboardElement;
        let state = {
            let mut s = AppState::default();
            s.dashboard.mark_starting("echo", Some("Echoes messages back".to_string()));
            s.dashboard.mark_starting("llm", Some("LLM streaming".to_string()));
            s
        };

        // When rendering with enough height.
        let rows = render_rows(&mut element, &state, 40, 10);

        // Then there is a blank line between the two actors.
        // echo on row 0, description on row 1, blank on row 2, llm on row 3.
        assert!(rows[0].contains("echo"));
        assert!(rows[1].contains("Echoes messages back"));
        assert!(rows[3].contains("llm"));
        assert!(rows[4].contains("LLM streaming"));
    }
}
