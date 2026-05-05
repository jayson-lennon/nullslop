//! Layout computation and rendering for the application.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui_tabs::{TabManager, TabsBar, TabsStyle};
use ratatui_which_key::{PopupPosition, WhichKey};

use crate::TuiApp;
use nullslop_protocol::{Mode, PickerKind};

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
        render_picker(frame, area, &state);
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

/// Renders the active picker overlay, dispatching on [`PickerKind`].
fn render_picker(frame: &mut Frame<'_>, area: Rect, state: &nullslop_component::AppState) {
    match state.active_picker_kind {
        Some(PickerKind::Provider) => render_provider_picker(frame, area, state),
        None => {} // Shouldn't happen — mode is Picker but no kind set
    }
}

/// Renders the provider picker overlay using [`SelectionWidget`].
///
/// Telescope-style layout: bordered popup with filter input at top,
/// horizontal separator, scrollable results, and a footer line.
fn render_provider_picker(frame: &mut Frame<'_>, area: Rect, state: &nullslop_component::AppState) {
    use nullslop_component::provider_picker::entries;
    use nullslop_selection_widget::SelectionWidget;

    let footer = entries::format_footer(state.last_refreshed_at.as_ref(), area.width as usize);
    let widget = SelectionWidget::new(&state.provider_picker)
        .title(ratatui::text::Line::from(" Model "))
        .footer(footer);
    widget.render(frame, area);
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

    fn picker_state_with_ollama() -> (nullslop_component::AppState, nullslop_services::Services) {
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
        (nullslop_component::AppState::default(), services)
    }

    /// Helper to load provider entries into the picker state.
    fn load_picker_items(
        state: &mut nullslop_component::AppState,
        services: &nullslop_services::Services,
    ) {
        nullslop_component::provider_picker::load_provider_picker_items(services, state);
    }

    #[test]
    fn render_provider_picker_shows_telescope_layout() {
        // Given a terminal area and picker state with filter "ol".
        use nullslop_selection_widget::compute_popup_rect;
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let (mut state, services) = picker_state_with_ollama();
        state.mode = Mode::Picker;
        state.active_picker_kind = Some(PickerKind::Provider);
        load_picker_items(&mut state, &services);
        state.provider_picker.insert_char('o');
        state.provider_picker.insert_char('l');

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
        use nullslop_selection_widget::compute_popup_rect;

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
        use nullslop_selection_widget::compute_popup_rect;
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let (mut state, services) = picker_state_with_ollama();
        load_picker_items(&mut state, &services);

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
        // Given a state with active_provider set to "ollama/llama3" and items loaded.
        use nullslop_selection_widget::compute_popup_rect;
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let (mut state, services) = picker_state_with_ollama();
        state.mode = Mode::Picker;
        state.active_provider = "ollama/llama3".to_owned();
        state.active_picker_kind = Some(PickerKind::Provider);
        load_picker_items(&mut state, &services);

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
