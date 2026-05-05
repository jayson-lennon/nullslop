//! Renders the dashboard view — a list of actors with their startup status.
//!
//! Each actor is displayed with a 2-cell left border. The selected entry shows
//! a solid yellow full block (`██`) in the border; unselected entries show spaces.
//! The view scrolls when actors overflow the viewport, keeping the selected
//! entry visible.

use crate::AppState;
use crate::dashboard::state::ActorStatus;
use nullslop_component_ui::UiElement;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

/// Solid yellow full block used as the selection indicator.
const SELECTED_INDICATOR: &str = "\u{2588}\u{2588}";
/// Two spaces used as the unselected border.
const UNSELECTED_BORDER: &str = "  ";

/// Display element for the actor dashboard.
#[derive(Debug)]
pub struct DashboardElement;

impl UiElement<AppState> for DashboardElement {
    fn name(&self) -> String {
        "dashboard".to_owned()
    }

    fn is_selectable(&self) -> bool {
        true
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, state: &AppState) {
        let actors = state.dashboard.actors();
        let selected_index = state.dashboard.selected_index();

        let lines: Vec<Line> = if actors.is_empty() {
            vec![Line::from(Span::styled(
                "No actors registered.",
                Style::default().fg(Color::DarkGray),
            ))]
        } else {
            let mut lines = Vec::new();

            for (i, entry) in actors.iter().enumerate() {
                let is_selected = i == selected_index;
                let border_span = if is_selected {
                    Span::styled(SELECTED_INDICATOR, Style::default().fg(Color::Yellow))
                } else {
                    Span::raw(UNSELECTED_BORDER)
                };

                let (label, color) = match entry.status {
                    ActorStatus::Starting => ("Starting", Color::Yellow),
                    ActorStatus::Running => ("Running", Color::Green),
                };

                // Name line: border + padded name ... status
                lines.push(Line::from(vec![
                    border_span,
                    Span::styled(
                        format!(" {}", entry.name),
                        Style::default().bold(),
                    ),
                    // Fill with spaces to push status right
                    Span::raw(fill_to_status(&entry.name, area.width)),
                    Span::styled(label, Style::default().fg(color)),
                ]));

                // Description line (if present).
                if let Some(desc) = &entry.description {
                    let desc_border = if is_selected {
                        Span::styled(SELECTED_INDICATOR, Style::default().fg(Color::Yellow))
                    } else {
                        Span::raw(UNSELECTED_BORDER)
                    };
                    lines.push(Line::from(vec![
                        desc_border,
                        Span::styled(
                            format!("   {desc}"),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]));
                }

                // Blank line between actors (not after the last one).
                if i < actors.len() - 1 {
                    lines.push(Line::from(""));
                }
            }

            lines
        };

        // Calculate total visual lines for scroll clamping.
        let total_lines = lines.len() as u16;
        let max_offset = total_lines.saturating_sub(area.height);
        let scroll_offset = state.dashboard.scroll_offset().min(max_offset);

        let widget = Paragraph::new(lines)
            .block(Block::default().borders(Borders::NONE))
            .scroll((scroll_offset, 0));
        frame.render_widget(widget, area);
    }
}

/// Returns spaces to pad between the name and the right-aligned status.
/// The status label takes up to ~8 chars ("Starting"), so we leave room.
/// The 2-cell left border is accounted for in the calculation.
fn fill_to_status(name: &str, area_width: u16) -> String {
    let status_width: usize = 8; // "Starting" is the longest status
    let border_width: usize = 2; // "██" or "  "
    let name_len = name.len() + 1 + border_width; // +1 for leading space, +2 for border
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
            s.dashboard.mark_starting("echo", Some("Echoes messages back".to_owned()));
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
            s.dashboard.mark_starting("echo", Some("Echoes messages back".to_owned()));
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
            s.dashboard.mark_starting("echo", Some("Echoes messages back".to_owned()));
            s.dashboard.mark_starting("llm", Some("LLM streaming".to_owned()));
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

    #[test]
    fn render_selected_entry_shows_yellow_block() {
        // Given two actors with the first selected (default index 0).
        let mut element = DashboardElement;
        let state = {
            let mut s = AppState::default();
            s.dashboard.mark_starting("echo", Some("Echoes messages back".to_owned()));
            s.dashboard.mark_starting("llm", Some("LLM streaming".to_owned()));
            s
        };

        // When rendering.
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let area = Rect::new(0, 0, 40, 10);
        terminal
            .draw(|frame| {
                element.render(frame, area, &state);
            })
            .unwrap();

        // Then the first entry has a yellow full block at columns 0-1.
        let buffer = terminal.backend().buffer();
        let cell0 = buffer.cell((0, 0)).expect("cell 0,0");
        let cell1 = buffer.cell((1, 0)).expect("cell 1,0");
        assert_eq!(cell0.symbol(), "\u{2588}");
        assert_eq!(cell0.fg, Color::Yellow);
        assert_eq!(cell1.symbol(), "\u{2588}");
        assert_eq!(cell1.fg, Color::Yellow);

        // And the description line also has the yellow block.
        let desc_cell0 = buffer.cell((0, 1)).expect("cell 0,1");
        assert_eq!(desc_cell0.symbol(), "\u{2588}");
        assert_eq!(desc_cell0.fg, Color::Yellow);
    }

    #[test]
    fn render_unselected_entry_shows_spaces() {
        // Given two actors with the first selected (default index 0).
        let mut element = DashboardElement;
        let state = {
            let mut s = AppState::default();
            s.dashboard.mark_starting("echo", Some("Echoes messages back".to_owned()));
            s.dashboard.mark_starting("llm", Some("LLM streaming".to_owned()));
            s
        };

        // When rendering.
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let area = Rect::new(0, 0, 40, 10);
        terminal
            .draw(|frame| {
                element.render(frame, area, &state);
            })
            .unwrap();

        // Then the second entry (row 3) has spaces at columns 0-1.
        let buffer = terminal.backend().buffer();
        let cell0 = buffer.cell((0, 3)).expect("cell 0,3");
        let cell1 = buffer.cell((1, 3)).expect("cell 1,3");
        assert_eq!(cell0.symbol(), " ");
        assert_eq!(cell1.symbol(), " ");
    }

    #[test]
    fn render_selection_moves_with_next() {
        // Given two actors with the second selected.
        let mut element = DashboardElement;
        let state = {
            let mut s = AppState::default();
            s.dashboard.mark_starting("echo", Some("Echoes messages back".to_owned()));
            s.dashboard.mark_starting("llm", Some("LLM streaming".to_owned()));
            s.dashboard.select_next();
            s
        };

        // When rendering.
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let area = Rect::new(0, 0, 40, 10);
        terminal
            .draw(|frame| {
                element.render(frame, area, &state);
            })
            .unwrap();

        // Then the first entry has spaces (unselected).
        let buffer = terminal.backend().buffer();
        let first_cell = buffer.cell((0, 0)).expect("cell 0,0");
        assert_eq!(first_cell.symbol(), " ");

        // And the second entry (row 3) has the yellow block (selected).
        let second_cell = buffer.cell((0, 3)).expect("cell 0,3");
        assert_eq!(second_cell.symbol(), "\u{2588}");
        assert_eq!(second_cell.fg, Color::Yellow);
    }

    #[test]
    fn dashboard_element_is_selectable() {
        // Given a DashboardElement.
        let element = DashboardElement;

        // When calling is_selectable.
        let selectable: &dyn UiElement<AppState> = &element;

        // Then it returns true.
        assert!(selectable.is_selectable());
    }
}
