//! Selection widget — ratatui renderer for the picker popup.
//!
//! [`SelectionWidget`] renders a telescope-style popup overlay: a bordered block containing
//! a filter input row with a real cursor, a horizontal separator, scrollable result rows,
//! and an optional footer. Each result row is rendered via [`PickerItem::render_row`],
//! so the consumer controls all styling.
//!
//! The popup rectangle is computed by [`compute_popup_rect`] using configurable constants
//! for horizontal padding, minimum width, and maximum height fraction.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::{PickerItem, SelectionState};

/// Horizontal padding as a fraction of terminal width (10% each side).
pub const PICKER_H_PAD_FRAC: f32 = 0.10;
/// Minimum popup width in cells.
pub const PICKER_MIN_WIDTH: u16 = 30;
/// Maximum fraction of terminal height the picker popup may consume.
pub const PICKER_MAX_HEIGHT_FRAC: f32 = 0.75;

/// Filter prompt displayed before the user's input text.
const PROMPT: &str = "> ";

/// Computes the popup rectangle for the selection widget.
///
/// Uses ~20% total horizontal padding (10% each side) and positions the popup
/// in the top third of the terminal. Height scales with terminal size, capped
/// at [`PICKER_MAX_HEIGHT_FRAC`] of the terminal height.
#[must_use]
pub fn compute_popup_rect(area: Rect) -> Rect {
    let popup_width = ((f32::from(area.width) * (1.0 - 2.0 * PICKER_H_PAD_FRAC)).ceil() as u16)
        .max(PICKER_MIN_WIDTH)
        .min(area.width);

    // Layout: border(2) + input(1) + separator(1) + results(N) + footer(1)
    // Reserve at least 4 rows for the chrome, use up to 75% of terminal height.
    let max_body_rows = (f32::from(area.height) * PICKER_MAX_HEIGHT_FRAC).floor() as u16;
    let popup_height = (max_body_rows + 4).min(area.height);

    // Integer division is intentional — we're computing cell positions for centering.
    #[expect(clippy::integer_division, reason = "cell positions are integers")]
    let popup_x = area.width.saturating_sub(popup_width) / 2;
    #[expect(clippy::integer_division, reason = "cell positions are integers")]
    let popup_y = area.height.saturating_sub(popup_height) / 3; // bias toward top third

    Rect::new(popup_x, popup_y, popup_width, popup_height)
}

/// Configuration for rendering the selection popup.
///
/// Generic over any type implementing [`PickerItem`]. Use the builder pattern to
/// customize the title and footer, then call [`render`](SelectionWidget::render).
///
/// # Examples
///
/// ```ignore
/// let state = SelectionState::with_items(my_items);
/// let widget = SelectionWidget::new(&state)
///     .title(" Model ")
///     .footer(Line::from("CTRL+R to refresh"));
/// widget.render(frame, area);
/// ```
pub struct SelectionWidget<'a, T: PickerItem> {
    /// Title displayed in the popup border (e.g., `" Model "`).
    title: Line<'a>,
    /// The selection state to render.
    state: &'a SelectionState<T>,
    /// Optional footer line (e.g., "CTRL+R to refresh | Updated ...").
    footer: Option<Line<'a>>,
}

impl<'a, T: PickerItem> SelectionWidget<'a, T> {
    /// Creates a new widget rendering the given selection state.
    pub fn new(state: &'a SelectionState<T>) -> Self {
        Self {
            title: Line::from(""),
            state,
            footer: None,
        }
    }

    /// Sets the popup border title.
    #[must_use]
    pub fn title(mut self, title: Line<'a>) -> Self {
        self.title = title;
        self
    }

    /// Sets an optional footer line rendered at the bottom of the popup.
    #[must_use]
    pub fn footer(mut self, footer: Line<'a>) -> Self {
        self.footer = Some(footer);
        self
    }

    /// Renders the selection popup within the given frame area.
    ///
    /// Computes the popup rectangle, draws the bordered block, filter input,
    /// separator, scrollable result rows, and optional footer.
    /// Sets the cursor position for the filter input.
    pub fn render(self, frame: &mut Frame<'_>, area: Rect) {
        let popup_area = compute_popup_rect(area);

        // Bordered block with muted border.
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(self.title);
        frame.render_widget(block, popup_area);

        // Layout: input line -> separator -> results -> footer.
        let inner = {
            let b = Block::default().borders(Borders::ALL);
            b.inner(popup_area)
        };
        let [input_area, separator_area, results_area, footer_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .areas(inner);

        // Filter input with real cursor.
        let filter_text = format!("{}{}", PROMPT, self.state.filter());
        let filter_paragraph = Paragraph::new(filter_text).style(Style::default().fg(Color::White));
        frame.render_widget(filter_paragraph, input_area);
        let cursor_col = input_area.x + (PROMPT.len() + self.state.cursor_pos()) as u16;
        frame.set_cursor_position((cursor_col, input_area.y));

        // Separator line.
        let separator = "\u{2500}".repeat(separator_area.width as usize);
        let sep_paragraph = Paragraph::new(separator).style(Style::default().fg(Color::DarkGray));
        frame.render_widget(sep_paragraph, separator_area);

        // Results area — windowed display with scroll_offset.
        let max_visible = results_area.height as usize;
        let scroll_offset = self.state.scroll_offset();
        let selection = self.state.selection();
        let mut result_lines = Vec::with_capacity(max_visible);

        for row in 0..max_visible {
            let entry_idx = scroll_offset + row;
            if let Some(item) = self.state.filtered_item(entry_idx) {
                let is_selected = entry_idx == selection;
                result_lines.push(item.render_row(is_selected));
            } else {
                // Empty row to maintain fixed height.
                result_lines.push(Line::from(""));
            }
        }
        frame.render_widget(Paragraph::new(result_lines), results_area);

        // Footer: right-aligned in DarkGray, or empty row.
        let footer_paragraph = match &self.footer {
            Some(line) => Paragraph::new(line.clone())
                .style(Style::default().fg(Color::DarkGray))
                .right_aligned(),
            None => Paragraph::new(""),
        };
        frame.render_widget(footer_paragraph, footer_area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    /// A minimal item type for testing.
    #[derive(Debug)]
    struct TestItem {
        label: String,
    }

    impl TestItem {
        fn new(label: &str) -> Self {
            Self {
                label: label.to_owned(),
            }
        }
    }

    impl PickerItem for TestItem {
        fn display_label(&self) -> &str {
            &self.label
        }

        fn render_row(&self, is_selected: bool) -> Line<'static> {
            if is_selected {
                Line::from(format!("> {}", self.label))
            } else {
                Line::from(self.label.clone())
            }
        }
    }

    /// Creates a list of test items from the given labels.
    fn make_items(labels: &[&str]) -> Vec<TestItem> {
        labels.iter().map(|&l| TestItem::new(l)).collect()
    }

    // =========================================================================
    // Ported from render.rs provider picker tests
    // =========================================================================

    #[test]
    fn render_shows_telescope_layout() {
        // Given a selection state with filter text.
        let mut state = SelectionState::with_items(make_items(&["ollama"]));
        state.insert_char('o');
        state.insert_char('l');

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // When rendering the widget.
        terminal
            .draw(|frame| {
                let widget = SelectionWidget::new(&state);
                widget.render(frame, frame.area());
            })
            .unwrap();

        // Then the popup contains the filter prompt "> " and separator "─".
        let buffer = terminal.backend().buffer().clone();
        let popup = compute_popup_rect(Rect::new(0, 0, 80, 24));
        // Filter is on the first inner row: popup.y + 1
        let filter_y = popup.y + 1;
        let prompt_cell = buffer.cell((popup.x + 1, filter_y)).expect("prompt cell");
        assert_eq!(prompt_cell.symbol(), ">");

        // Separator is on the second inner row.
        let sep_y = popup.y + 2;
        let sep_cell = buffer.cell((popup.x + 1, sep_y)).expect("sep cell");
        assert_eq!(sep_cell.symbol(), "\u{2500}");
    }

    #[test]
    fn render_height_scales_with_terminal() {
        // Given two terminal sizes.
        let small_area = Rect::new(0, 0, 80, 24);
        let large_area = Rect::new(0, 0, 80, 42);

        // When computing popup rects.
        let small_popup = compute_popup_rect(small_area);
        let large_popup = compute_popup_rect(large_area);

        // Then the larger terminal gets a taller popup.
        assert!(large_popup.height > small_popup.height);

        // And the small terminal popup uses 75% of height + 4 rows of chrome.
        // floor(24 * 0.75) = 18, min(18 + 4, 24) = 22.
        assert_eq!(small_popup.height, 22);
    }

    #[test]
    fn render_uses_dark_gray_border() {
        // Given a widget with default state.
        let state = SelectionState::with_items(make_items(&["test"]));

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // When rendering the widget.
        terminal
            .draw(|frame| {
                let widget = SelectionWidget::new(&state);
                widget.render(frame, frame.area());
            })
            .unwrap();

        // Then the border color is DarkGray.
        let buffer = terminal.backend().buffer().clone();
        let popup = compute_popup_rect(Rect::new(0, 0, 80, 24));
        let border_cell = buffer.cell((popup.x, popup.y)).expect("border cell");
        assert_eq!(border_cell.fg, Color::DarkGray);
    }

    #[test]
    fn render_calls_render_row_for_selected_item() {
        // Given a selection state with items where the first is selected.
        let state = SelectionState::with_items(make_items(&["alpha", "bravo"]));

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // When rendering the widget.
        terminal
            .draw(|frame| {
                let widget = SelectionWidget::new(&state);
                widget.render(frame, frame.area());
            })
            .unwrap();

        // Then the first result row starts with "> " (render_row with is_selected=true).
        let buffer = terminal.backend().buffer().clone();
        let popup = compute_popup_rect(Rect::new(0, 0, 80, 24));
        // Results start at popup.y + 3 (border + input + separator)
        let result_y = popup.y + 3;
        let marker_cell = buffer.cell((popup.x + 1, result_y)).expect("marker cell");
        assert_eq!(marker_cell.symbol(), ">");
    }

    // =========================================================================
    // New widget-specific tests
    // =========================================================================

    #[test]
    fn render_shows_title() {
        // Given a widget with a custom title.
        let state = SelectionState::with_items(make_items(&["test"]));
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // When rendering with title " Model ".
        terminal
            .draw(|frame| {
                let widget = SelectionWidget::new(&state).title(Line::from(" Model "));
                widget.render(frame, frame.area());
            })
            .unwrap();

        // Then the title appears in the border area.
        let buffer = terminal.backend().buffer().clone();
        let popup = compute_popup_rect(Rect::new(0, 0, 80, 24));
        // Title is rendered on the top border row. ratatui renders the title
        // starting at a position along the top border line.
        let title_y = popup.y;
        // Find the 'M' from " Model " on the top border row.
        let mut found_title = false;
        for col in popup.x..popup.x + popup.width {
            if let Some(cell) = buffer.cell((col, title_y))
                && cell.symbol() == "M"
            {
                found_title = true;
                break;
            }
        }
        assert!(
            found_title,
            "expected to find 'M' from title ' Model ' on the top border row"
        );
    }

    #[test]
    fn render_shows_footer_when_provided() {
        // Given a widget with a footer.
        let state = SelectionState::with_items(make_items(&["test"]));
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // When rendering with footer text.
        let footer_text = "CTRL+R to refresh";
        terminal
            .draw(|frame| {
                let widget = SelectionWidget::new(&state).footer(Line::from(footer_text));
                widget.render(frame, frame.area());
            })
            .unwrap();

        // Then the footer content appears in the footer area.
        let buffer = terminal.backend().buffer().clone();
        let popup = compute_popup_rect(Rect::new(0, 0, 80, 24));
        let inner = {
            let b = Block::default().borders(Borders::ALL);
            b.inner(popup)
        };
        // Footer is the last row of the inner area.
        let footer_y = inner.y + inner.height - 1;

        // Footer is right-aligned, so search for "C" from "CTRL" somewhere on footer_y.
        let mut found_footer = false;
        for col in inner.x..inner.x + inner.width {
            if let Some(cell) = buffer.cell((col, footer_y))
                && cell.symbol() == "C"
            {
                found_footer = true;
                break;
            }
        }
        assert!(
            found_footer,
            "expected to find footer text on the footer row"
        );
    }

    #[test]
    fn render_no_footer_shows_empty_row() {
        // Given a widget without a footer.
        let state = SelectionState::with_items(make_items(&["test"]));
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // When rendering without footer.
        terminal
            .draw(|frame| {
                let widget = SelectionWidget::new(&state);
                widget.render(frame, frame.area());
            })
            .unwrap();

        // Then the footer area contains empty/spaces (no visible text).
        let buffer = terminal.backend().buffer().clone();
        let popup = compute_popup_rect(Rect::new(0, 0, 80, 24));
        let inner = {
            let b = Block::default().borders(Borders::ALL);
            b.inner(popup)
        };
        let footer_y = inner.y + inner.height - 1;

        // All cells in the footer row should be spaces (empty).
        for col in inner.x..inner.x + inner.width {
            if let Some(cell) = buffer.cell((col, footer_y)) {
                assert_eq!(
                    cell.symbol(),
                    " ",
                    "expected empty cell at ({}, {}), got '{}'",
                    col,
                    footer_y,
                    cell.symbol()
                );
            }
        }
    }

    #[test]
    fn render_pads_empty_result_rows() {
        // Given a selection state with only 1 item but many visible rows.
        let state = SelectionState::with_items(make_items(&["solo"]));
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // When rendering the widget.
        terminal
            .draw(|frame| {
                let widget = SelectionWidget::new(&state);
                widget.render(frame, frame.area());
            })
            .unwrap();

        // Then the results area is mostly empty rows (only the first result has content).
        let buffer = terminal.backend().buffer().clone();
        let popup = compute_popup_rect(Rect::new(0, 0, 80, 24));
        let inner = {
            let b = Block::default().borders(Borders::ALL);
            b.inner(popup)
        };
        // Results area starts at inner.y + 2 (after input and separator).
        let results_start_y = inner.y + 2;
        // Results area height = inner.height - 3 (input + separator + footer).
        let results_height = inner.height - 3;

        // First result row should have content.
        let first_cell = buffer
            .cell((inner.x, results_start_y))
            .expect("first result cell");
        // The item is "solo" and is selected, so render_row returns "> solo".
        assert_ne!(
            first_cell.symbol(),
            " ",
            "first result row should have content"
        );

        // Second result row should be empty (padded).
        if results_height > 1 {
            let second_cell = buffer
                .cell((inner.x, results_start_y + 1))
                .expect("second result cell");
            assert_eq!(
                second_cell.symbol(),
                " ",
                "second result row should be empty/padded"
            );
        }
    }

    #[test]
    fn render_positions_cursor_correctly() {
        // Given a selection state with filter text and cursor in the middle.
        let mut state = SelectionState::with_items(make_items(&["test"]));
        state.insert_char('a');
        state.insert_char('b');
        state.insert_char('c');
        // Filter is "abc", cursor at position 3 (end).
        // Move cursor back to position 1 (between 'a' and 'b').
        state.move_cursor_left();
        state.move_cursor_left();
        assert_eq!(state.cursor_pos(), 1);

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // When rendering the widget.
        terminal
            .draw(|frame| {
                let widget = SelectionWidget::new(&state);
                widget.render(frame, frame.area());
            })
            .unwrap();

        // Then the cursor is at the expected position within the input row.
        let popup = compute_popup_rect(Rect::new(0, 0, 80, 24));
        let inner = {
            let b = Block::default().borders(Borders::ALL);
            b.inner(popup)
        };
        // Cursor should be at: input_area.x + PROMPT.len() + cursor_pos = inner.x + 2 + 1 = inner.x + 3
        let expected_cursor_x = inner.x + (PROMPT.len() + 1) as u16;
        let expected_cursor_y = inner.y;

        // Verify the buffer cell at the cursor position shows 'b' (cursor is before 'b').
        let buffer = terminal.backend().buffer().clone();
        let cursor_cell = buffer
            .cell((expected_cursor_x, expected_cursor_y))
            .expect("cursor cell");
        assert_eq!(cursor_cell.symbol(), "b");
    }

    #[test]
    fn compute_popup_rect_width_clamps_to_min() {
        // Given a very narrow terminal (width 20, less than PICKER_MIN_WIDTH).
        let area = Rect::new(0, 0, 20, 24);

        // When computing the popup rect.
        let popup = compute_popup_rect(area);

        // Then popup width is PICKER_MIN_WIDTH, but cannot exceed area width.
        assert_eq!(
            popup.width,
            PICKER_MIN_WIDTH.min(area.width),
            "popup width should be clamped to min or terminal width"
        );
    }

    #[test]
    fn compute_popup_rect_cannot_exceed_terminal() {
        // Given a terminal area.
        let area = Rect::new(0, 0, 80, 24);

        // When computing the popup rect.
        let popup = compute_popup_rect(area);

        // Then popup width and height never exceed area dimensions.
        assert!(popup.width <= area.width, "popup width exceeds terminal");
        assert!(popup.height <= area.height, "popup height exceeds terminal");
        assert!(
            popup.x + popup.width <= area.width,
            "popup extends beyond right edge"
        );
        assert!(
            popup.y + popup.height <= area.height,
            "popup extends beyond bottom edge"
        );
    }

    #[test]
    fn compute_popup_rect_centers_horizontally() {
        // Given an 80-wide terminal.
        let area = Rect::new(0, 0, 80, 24);

        // When computing the popup rect.
        let popup = compute_popup_rect(area);

        // Then popup is horizontally centered (equal padding on both sides).
        let left_pad = popup.x;
        let right_pad = area.width - (popup.x + popup.width);
        // Allow off-by-one due to integer division.
        assert!(
            (i32::from(left_pad) - i32::from(right_pad)).unsigned_abs() <= 1,
            "popup should be roughly centered: left_pad={left_pad}, right_pad={right_pad}"
        );
    }

    #[test]
    fn compute_popup_rect_biased_to_top_third() {
        // Given a tall terminal.
        let area = Rect::new(0, 0, 80, 60);

        // When computing the popup rect.
        let popup = compute_popup_rect(area);

        // Then popup is positioned in the top third (y < height / 3).
        #[expect(clippy::integer_division, reason = "cell positions are integers")]
        let area_third = area.height / 3;
        assert!(
            popup.y < area_third,
            "popup y ({}) should be in the top third (below {})",
            popup.y,
            area_third
        );
    }
}
