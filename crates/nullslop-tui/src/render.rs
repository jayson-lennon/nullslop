//! Layout computation and rendering for the application.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui_tabs::{TabManager, TabsBar, TabsStyle};
use ratatui_which_key::{PopupPosition, WhichKey};

use crate::TuiApp;
use nullslop_component::provider_picker::entries::{filtered_entries, sorted_entries};
use nullslop_protocol::Mode;

/// Minimum terminal width.
pub const MIN_WIDTH: u16 = 40;
/// Minimum terminal height.
pub const MIN_HEIGHT: u16 = 13;

/// Top-level application layout areas.
pub struct AppLayout {
    /// The tab bar area (1 row at top).
    pub tabs: Rect,
    /// The main content area (fills remaining space).
    pub content: Rect,
    /// The streaming indicator area (1 row between content and counter).
    pub indicator: Rect,
    /// The queue display area (dynamic height based on queue length).
    pub queue: Rect,
    /// The character counter area (1 row above input, chat tab only).
    pub counter: Rect,
    /// The input box area (3 rows at bottom, chat tab only).
    pub input: Rect,
}

impl AppLayout {
    /// Returns `true` if the given area meets minimum size requirements.
    #[must_use]
    pub const fn meets_min_size(area: Rect) -> bool {
        area.width >= MIN_WIDTH && area.height >= MIN_HEIGHT
    }

    /// Computes the layout for the given terminal area.
    ///
    /// `input_lines` is the number of visual lines the input box needs
    /// (used for dynamic multi-line input height).
    ///
    /// `queue_lines` is the number of rows for the queue display area
    /// (0 when queue is empty).
    #[must_use]
    pub fn new(area: Rect, input_lines: u16, queue_lines: u16) -> Self {
        let [tabs, rest] =
            Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);

        let input_height = 1 + input_lines.max(1); // top border + at least 1 line
        let [content, indicator, queue, counter, input] = Layout::vertical([
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(queue_lines),
            Constraint::Length(1),
            Constraint::Length(input_height),
        ])
        .areas(rest);

        Self {
            tabs,
            content,
            indicator,
            queue,
            counter,
            input,
        }
    }
}

/// Build the default tab manager with Chat and Dashboard tabs.
pub fn init_tab_manager() -> TabManager {
    let mut mgr = TabManager::new();
    mgr.add_tab("Chat");
    mgr.add_tab("Dashboard");
    mgr
}

/// Renders the full application frame.
pub fn render(app: &mut TuiApp, frame: &mut Frame<'_>) {
    let area = frame.area();
    if !AppLayout::meets_min_size(area) {
        render_too_small(frame, area);
        return;
    }

    let state = app.core.state.read();
    let queue_len = state.active_session().queue_len() as u16;
    let layout = AppLayout::new(
        area,
        state.active_chat_input().visual_line_count() as u16,
        queue_len,
    );

    // Tab bar — always visible.
    render_tab_bar(frame, layout.tabs, &app.tab_manager);

    match state.active_tab {
        nullslop_protocol::ActiveTab::Chat => {
            // Chat log
            if let Some(element) = app.ui_registry.get_mut("chat-log") {
                element.render(frame, layout.content, &state);
            }
            // Streaming indicator (dedicated row between content and input)
            if let Some(element) = app.ui_registry.get_mut("streaming-indicator") {
                element.render(frame, layout.indicator, &state);
            }
            // Queue display
            if let Some(element) = app.ui_registry.get_mut("queue-display") {
                element.render(frame, layout.queue, &state);
            }
            // Character counter
            if let Some(element) = app.ui_registry.get_mut("char-counter") {
                element.render(frame, layout.counter, &state);
            }
            // Input box
            if let Some(element) = app.ui_registry.get_mut("chat-input-box") {
                element.render(frame, layout.input, &state);
            }
        }
        nullslop_protocol::ActiveTab::Dashboard => {
            // Dashboard fills the entire content area
            if let Some(element) = app.ui_registry.get_mut("dashboard") {
                element.render(frame, layout.content, &state);
            }
        }
    }

    // Which-key popup overlay (app-level, not a component element)
    render_which_key(frame, &mut app.which_key);

    if state.mode == Mode::Picker {
        render_provider_picker(frame, area, &state);
    }
}

/// Renders the tab bar.
fn render_tab_bar(frame: &mut Frame<'_>, area: Rect, manager: &TabManager) {
    let tabs = manager.tabs();
    let active_id = manager.active_id();
    let bar = TabsBar::new(tabs, active_id).tabs_style(TabsStyle {
        active: Style::default().fg(Color::Black).bg(Color::Yellow),
        inactive: Style::default().fg(Color::Gray),
        ..TabsStyle::default()
    });
    frame.render_widget(bar, area);
}

/// Renders the which-key popup overlay.
fn render_which_key(frame: &mut Frame<'_>, state: &mut crate::app::WhichKeyInstance) {
    let widget = WhichKey::new()
        .position(PopupPosition::BottomRight)
        .border_style(Style::default().fg(Color::Yellow));
    let buf = frame.buffer_mut();
    widget.render(buf, state);
}

/// Horizontal padding as a fraction of terminal width (10% each side = 20% total).
const PICKER_H_PAD_FRAC: f32 = 0.10;
/// Minimum popup width.
const PICKER_MIN_WIDTH: u16 = 30;
/// Maximum fraction of terminal height the picker popup may consume.
const PICKER_MAX_HEIGHT_FRAC: f32 = 0.75;

/// Computes the popup rectangle for the provider picker.
///
/// Uses ~20% total horizontal padding (10% each side) and positions the popup
/// in the top third of the terminal. Height scales with terminal size.
#[must_use]
fn compute_popup_rect(area: Rect) -> Rect {
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

/// Renders the provider picker overlay.
///
/// Telescope-style layout: bordered popup with filter input at top,
/// horizontal separator, scrollable results, and a footer line.
fn render_provider_picker(frame: &mut Frame<'_>, area: Rect, state: &nullslop_component::AppState) {
    use ratatui::widgets::{Block, Borders};

    let popup_area = compute_popup_rect(area);

    let services = &state.services;
    let registry = services.provider_registry().read();
    let api_keys = services.api_keys().read();
    let unsorted = filtered_entries(&registry, &api_keys, &state.picker.filter, state.model_cache.as_ref());
    let entries = sorted_entries(
        &unsorted,
        &state.picker.filter,
        &state.active_provider,
    );

    // Render popup block with muted border.
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Provider ");
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
    let prompt = "> ";
    let filter_text = format!("{}{}", prompt, state.picker.filter);
    let filter_paragraph = Paragraph::new(filter_text).style(Style::default().fg(Color::White));
    frame.render_widget(filter_paragraph, input_area);
    let cursor_col = input_area.x + (prompt.len() + state.picker.cursor_pos()) as u16;
    frame.set_cursor_position((cursor_col, input_area.y));

    // Separator line.
    let separator = "\u{2500}".repeat(separator_area.width as usize);
    let sep_paragraph = Paragraph::new(separator).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(sep_paragraph, separator_area);

    // Results area \u2014 windowed display with scroll_offset.
    let max_visible = results_area.height as usize;
    let result_lines = build_result_lines(&entries, &state.picker, &state.active_provider, max_visible);
    frame.render_widget(Paragraph::new(result_lines), results_area);

    // Footer: last updated timestamp + refresh hint.
    let footer_text = format_footer(state.last_refreshed_at.as_ref(), footer_area.width as usize);
    let footer_paragraph = Paragraph::new(footer_text).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer_paragraph, footer_area);
}

/// Builds the result lines for the picker, using scroll offset for windowed display.
///
/// Empty rows are added when there are fewer entries than `max_visible`
/// to maintain a fixed-height popup.
fn build_result_lines<'a>(
    entries: &[nullslop_component::provider_picker::entries::PickerEntry],
    picker: &nullslop_component::provider_picker::ProviderPickerState,
    active_provider: &str,
    max_visible: usize,
) -> Vec<ratatui::text::Line<'a>> {
    use ratatui::style::Modifier;
    use ratatui::text::{Line, Span};

    let scroll_offset = picker.scroll_offset();
    let selection = picker.selection;
    let mut lines = Vec::with_capacity(max_visible);

    for row in 0..max_visible {
        let entry_idx = scroll_offset + row;
        if let Some(entry) = entries.get(entry_idx) {
            let is_selected = entry_idx == selection;
            let is_active = entry.provider_id == active_provider;

            let active_marker = Span::styled(
                if is_active { "> " } else { "  " },
                if is_active {
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                },
            );

            let status = if !entry.is_available {
                "\u{2717} " // ✗
            } else if entry.is_alias {
                "\u{2192} " // →
            } else if entry.is_remote {
                "* "
            } else {
                "  "
            };

            let label = if entry.is_alias {
                format!(
                    "{}{} \u{2192} {} ({})",
                    status, entry.name, entry.model, entry.provider_name
                )
            } else {
                format!("{}{} ({})", status, entry.model, entry.provider_name)
            };

            let label_style = if is_selected {
                Style::default().fg(Color::White).bg(Color::DarkGray)
            } else if !entry.is_available {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };

            lines.push(Line::from(vec![active_marker, Span::styled(label, label_style)]));
        } else {
            // Empty row to maintain fixed height.
            lines.push(Line::from(""));
        }
    }

    lines
}

/// Formats the footer text showing last updated time and refresh hint.
fn format_footer(last_refreshed_at: Option<&std::time::Instant>, width: usize) -> String {
    let timestamp = match last_refreshed_at {
        Some(instant) => {
            let elapsed = instant.elapsed();
            format_relative_time(elapsed)
        }
        None => "never".to_owned(),
    };
    let footer = format!("Last updated: {timestamp}   CTRL+R to refresh");
    // Truncate to fit width.
    if footer.len() > width {
        footer.chars().take(width).collect()
    } else {
        footer
    }
}

/// Formats a duration as a human-readable relative time string.
fn format_relative_time(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{secs}s ago")
    } else if secs < 3600 {
        #[expect(clippy::integer_division, reason = "intentional truncating division for minute display")]
        let minutes = secs / 60;
        format!("{minutes}m ago")
    } else {
        #[expect(clippy::integer_division, reason = "intentional truncating division for hour display")]
        let hours = secs / 3600;
        format!("{hours}h ago")
    }
}

/// Renders a "terminal too small" message.
fn render_too_small(frame: &mut Frame<'_>, area: Rect) {
    let msg = format!("Terminal too small\n{MIN_WIDTH}x{MIN_HEIGHT} minimum");
    let paragraph = Paragraph::new(msg).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_layout_meets_min_size() {
        // Given a 40x13 area.
        let area = Rect::new(0, 0, 40, 13);

        // When checking meets_min_size.
        let result = AppLayout::meets_min_size(area);

        // Then it returns true.
        assert!(result);
    }

    #[test]
    fn app_layout_too_small() {
        // Given a 10x5 area.
        let area = Rect::new(0, 0, 10, 5);

        // When checking meets_min_size.
        let result = AppLayout::meets_min_size(area);

        // Then it returns false.
        assert!(!result);
    }

    #[test]
    fn init_tab_manager_has_two_tabs() {
        // Given a default tab manager.
        let mgr = init_tab_manager();

        // When checking tab count.
        // Then there are 2 tabs and the first is active.
        assert_eq!(mgr.tab_count(), 2);
        assert!(mgr.active_tab().is_some());
        assert_eq!(mgr.active_tab().unwrap().name, "Chat");
    }

    #[test]
    fn app_layout_includes_indicator_row() {
        // Given a 40x13 area.
        let area = Rect::new(0, 0, 40, 13);
        let layout = AppLayout::new(area, 1, 0);

        // Then the indicator row has height 1 and is between content and counter.
        assert_eq!(layout.indicator.height, 1);
        assert!(layout.indicator.y > layout.content.y);
        assert!(layout.indicator.y < layout.counter.y);
    }

    #[test]
    fn app_layout_queue_area_has_dynamic_height() {
        // Given a 40x20 area with 3 queued messages.
        let area = Rect::new(0, 0, 40, 20);
        let layout = AppLayout::new(area, 1, 3);

        // Then the queue area has height 3 and sits between indicator and counter.
        assert_eq!(layout.queue.height, 3);
        assert!(layout.queue.y > layout.indicator.y);
        assert!(layout.queue.y < layout.counter.y);
    }

    #[test]
    fn app_layout_queue_area_zero_height_when_empty() {
        // Given a 40x13 area with no queued messages.
        let area = Rect::new(0, 0, 40, 13);
        let layout = AppLayout::new(area, 1, 0);

        // Then the queue area has height 0.
        assert_eq!(layout.queue.height, 0);
    }

    // --- Provider picker rendering tests ---

    fn picker_state_with_ollama() -> nullslop_component::AppState {
        use nullslop_providers::{ProviderEntry, ProvidersConfig};
        let config = ProvidersConfig {
            providers: vec![ProviderEntry {
                name: "ollama".to_owned(),
                backend: "ollama".to_owned(),
                models: vec!["llama3".to_owned()],
                base_url: Some("http://localhost:11434".to_owned()),
                api_key_env: None,
                requires_key: false,
            }],
            aliases: vec![],
            default_provider: None,
        };
        let services = nullslop_services::test_services::TestServices::builder()
            .with_providers(config)
            .build();
        nullslop_component::AppState::new(services)
    }

    #[test]
    fn render_provider_picker_shows_telescope_layout() {
        // Given a terminal area and picker state with filter "ol".
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let mut state = picker_state_with_ollama();
        state.mode = nullslop_protocol::Mode::Picker;
        state.picker.filter = "ol".to_owned();

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // When rendering the picker.
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_provider_picker(frame, area, &state);
            })
            .unwrap();

        // Then the popup contains the filter text with "> ol".
        let buffer = terminal.backend().buffer().clone();
        let popup = compute_popup_rect(Rect::new(0, 0, 80, 24));
        // Filter is on the first inner row: popup.y + 1
        let filter_y = popup.y + 1;
        let filter_cell = buffer.cell((popup.x + 1, filter_y)).expect("filter cell");
        assert_eq!(filter_cell.symbol(), ">");

        // Separator is on the second inner row.
        let sep_y = popup.y + 2;
        let sep_cell = buffer.cell((popup.x + 1, sep_y)).expect("sep cell");
        assert_eq!(sep_cell.symbol(), "\u{2500}");
    }

    #[test]
    fn render_provider_picker_height_scales_with_terminal() {
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
    fn render_provider_picker_uses_dark_gray_border() {
        // Given a picker render.
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let state = picker_state_with_ollama();

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_provider_picker(frame, area, &state);
            })
            .unwrap();

        // Then the border color is DarkGray, not Yellow.
        let buffer = terminal.backend().buffer().clone();
        let popup = compute_popup_rect(Rect::new(0, 0, 80, 24));
        let border_cell = buffer.cell((popup.x, popup.y)).expect("border cell");
        assert_eq!(border_cell.fg, Color::DarkGray);
    }

    #[test]
    fn render_provider_picker_shows_active_model_marker() {
        // Given a state with active_provider set to "ollama/llama3" and empty filter.
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let mut state = picker_state_with_ollama();
        state.mode = nullslop_protocol::Mode::Picker;
        state.active_provider = "ollama/llama3".to_owned();

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // When rendering the picker.
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_provider_picker(frame, area, &state);
            })
            .unwrap();

        // Then the first result row starts with ">" (active marker) in green.
        let buffer = terminal.backend().buffer().clone();
        let popup = compute_popup_rect(Rect::new(0, 0, 80, 24));
        // Results start at popup.y + 3 (border + input + separator)
        let result_y = popup.y + 3;
        let marker_cell = buffer.cell((popup.x + 1, result_y)).expect("marker cell");
        assert_eq!(marker_cell.symbol(), ">");
        assert_eq!(marker_cell.fg, Color::Green);
    }
}
