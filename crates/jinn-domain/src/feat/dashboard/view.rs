//! The dashboard's VIEW artifact — draws the dashboard slice payload.
//!
//! Third of the contribution triple (STATE = [`DashboardState`] cell,
//! LOGIC = [`DashboardCanvasActor`](super::canvas_actor::DashboardCanvasActor),
//! VIEW = [`DashboardView`]): a pure renderer over `&DashboardState`
//! plus the current theme, which stays in `AppState` because themes are
//! runtime-switchable application data, not slice payload.

use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, HighlightSpacing, Paragraph, Row, Table, TableState};

use crate::common::slices::SlotKey;
use crate::common::slices::view::SliceView;
use crate::common::slices::view::ViewCx;
use crate::feat::dashboard::ActorLifecycle;
use crate::feat::dashboard::DashboardEntry;
use crate::feat::dashboard::DashboardState;
use crate::feat::theme::Theme;

/// Renders the dashboard slice: one row per actor, selection highlight,
/// scroll window. Owns no domain state — every drawn value comes from
/// the slice and the theme.
///
/// The theme comes in through [`ViewCx`] — it is application data
/// (runtime-switchable), not slice payload.
#[derive(Debug)]
pub struct DashboardView {
    slot: SlotKey,
}

impl DashboardView {
    /// A view over the dashboard's canonical slice slot.
    #[must_use]
    pub fn new() -> Self {
        Self {
            slot: super::dashboard_slot(),
        }
    }
}

impl Default for DashboardView {
    fn default() -> Self {
        Self::new()
    }
}

impl SliceView for DashboardView {
    type Slice = DashboardState;

    fn slot(&self) -> SlotKey {
        self.slot.clone()
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, cx: &ViewCx<'_>, slice: &Self::Slice) {
        let theme = cx.theme;
        let actors = slice.actors();
        if actors.is_empty() {
            render_empty(frame, area, theme);
            return;
        }

        let rows = build_rows(&actors, theme);
        let widths = [
            Constraint::Length(22),
            Constraint::Min(10),
            Constraint::Length(10),
            Constraint::Min(10),
        ];

        let header = Row::new(vec![
            Cell::from("Name"),
            Cell::from("Description"),
            Cell::from("State"),
            Cell::from("Notes"),
        ])
        .style(Style::default().add_modifier(Modifier::BOLD));

        let table = Table::new(rows, widths)
            .header(header)
            .column_spacing(2)
            .row_highlight_style(Style::default().fg(theme.focus_accent))
            .highlight_symbol("▸ ")
            .highlight_spacing(HighlightSpacing::Always);

        let mut table_state = TableState::default();
        table_state.select(Some(slice.selected_index()));
        *table_state.offset_mut() = usize::from(slice.scroll_offset());

        frame.render_stateful_widget(table, area, &mut table_state);
    }
}

/// Renders the empty-state placeholder.
fn render_empty(frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
    let para = Paragraph::new(Line::from(Span::styled(
        " No services registered.",
        Style::default().fg(theme.muted_text),
    )));
    frame.render_widget(para, area);
}

/// Builds the table rows from dashboard entries, applying per-lifecycle colors.
fn build_rows<'a>(actors: &[&'a DashboardEntry], theme: &Theme) -> Vec<Row<'a>> {
    actors
        .iter()
        .map(|entry| {
            let name_cell =
                Cell::from(entry.name.as_str()).style(Style::default().fg(theme.primary_text));

            let desc_cell = Cell::from(entry.description.as_deref().unwrap_or(""))
                .style(Style::default().fg(theme.muted_text));

            let (state_str, state_color) = lifecycle_display(entry.lifecycle, theme);
            let state_cell = Cell::from(state_str).style(Style::default().fg(state_color));

            let status_str = entry.status_message.as_deref().unwrap_or("");
            let status_cell = Cell::from(status_str).style(Style::default().fg(theme.muted_text));

            Row::new(vec![name_cell, desc_cell, state_cell, status_cell])
        })
        .collect()
}

/// Returns the display string and color for a lifecycle variant.
fn lifecycle_display(lifecycle: ActorLifecycle, theme: &Theme) -> (&'static str, Color) {
    match lifecycle {
        ActorLifecycle::Starting => ("Starting", theme.warning),
        ActorLifecycle::Running => ("Running", theme.success),
        ActorLifecycle::Dead => ("Dead", theme.error_text),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::indexing_slicing, reason = "test code")]
    use super::DashboardState;
    use super::DashboardView;
    use super::SliceView;
    use super::ViewCx;
    use jinn_testutil::setup_term;

    /// Collects the entire terminal buffer into a single string for substring
    /// assertions.
    fn buffer_string(terminal: &ratatui::Terminal<ratatui::backend::TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect()
    }

    #[rstest::rstest]
    #[test]
    fn dashboard_view_renders_rows_with_selection() {
        // Given a dashboard slice with two actors, the second selected.
        let mut slice = DashboardState::new();
        slice.mark_running("alpha", None);
        slice.mark_running("beta", Some("second".to_owned()));
        slice.select_next();
        let theme = crate::feat::theme::default_theme();
        let cx = ViewCx { theme: &theme };

        // When rendering through the view.
        let (mut terminal, _area) = setup_term(80, 24);
        terminal
            .draw(|frame| {
                let mut view = DashboardView::new();
                let area = ratatui::layout::Rect::new(0, 0, 80, 24);
                view.render(frame, area, &cx, &slice);
            })
            .expect("render");

        // Then both rows are drawn and the marker sits on `beta`.
        let buf = buffer_string(&terminal);
        assert!(buf.contains("alpha"), "first row renders");
        assert!(buf.contains("beta"), "second row renders");
        assert!(buf.contains('▸'), "selection marker renders");
        assert!(buf.contains("Running"), "lifecycle column renders");
    }

    #[rstest::rstest]
    #[test]
    fn dashboard_view_renders_empty_placeholder_when_no_actors() {
        // Given an empty dashboard slice.
        let slice = DashboardState::new();
        let theme = crate::feat::theme::default_theme();
        let cx = ViewCx { theme: &theme };

        // When rendering through the view.
        let (mut terminal, _area) = setup_term(80, 24);
        terminal
            .draw(|frame| {
                let mut view = DashboardView::new();
                let area = ratatui::layout::Rect::new(0, 0, 80, 24);
                view.render(frame, area, &cx, &slice);
            })
            .expect("render");

        // Then the placeholder is drawn.
        assert!(buffer_string(&terminal).contains("No services"));
    }

    #[rstest::rstest]
    #[test]
    fn clamp_scroll_keeps_selected_visible() {
        // Given a dashboard with 5 actors, selection at index 4, viewport 3.
        let mut state = DashboardState::new();
        for name in ["a", "b", "c", "d", "e"] {
            state.mark_running(name, None);
        }
        state.select_last(); // index 4
        assert_eq!(state.selected_index(), 4);

        // When clamping with viewport 3.
        state.clamp_scroll(3);

        // Then scroll_offset puts index 4 within the visible window.
        let visible_start = state.scroll_offset() as usize;
        let visible_end = visible_start + 3;
        assert!(
            (visible_start..visible_end).contains(&4),
            "selected index should be within visible window {visible_start}..{visible_end}"
        );
    }
}
