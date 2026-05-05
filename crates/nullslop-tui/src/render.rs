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
        render_too_small(frame, area, app);
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

    // Collect selectable rects during rendering.
    let mut rects = vec![];

    match state.active_tab {
        nullslop_protocol::ActiveTab::Chat => {
            // Chat log
            if let Some(element) = app.ui_registry.get_mut("chat-log") {
                element.render(frame, layout.content, &state);
                if element.is_selectable() {
                    rects.push(layout.content);
                }
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
                if element.is_selectable() {
                    rects.push(layout.content);
                }
            }
        }
    }

    // Which-key popup overlay (app-level, not a component element)
    render_which_key(frame, &mut app.which_key);

    if state.mode == Mode::Picker {
        render_picker(frame, area, &state);
        // Provider picker popup is selectable — not a UiElement, register inline.
        rects.push(nullslop_selection_widget::compute_popup_rect(area));
    }

    // Release the state read lock before clipboard flush needs &mut app.
    drop(state);

    app.selectable_rects.rebuild(rects);

    // Apply selection highlight after all elements have rendered.
    apply_selection_highlight(app, frame.buffer_mut());

    // Flush pending clipboard copy (reads buffer, writes system clipboard).
    flush_pending_clipboard(app, frame.buffer_mut());
}

/// Inverts foreground and background for cells within the active selection rect.
///
/// This is a post-rendering pass applied after all UI elements have drawn.
/// The selection rect comes from [`SelectionState::selection_rect()`], which is
/// already normalized and clamped to the constraining bounds.
fn apply_selection_highlight(app: &TuiApp, buf: &mut ratatui::buffer::Buffer) {
    if let Some(sel_rect) = app.selection.selection_rect() {
        for y in sel_rect.top()..sel_rect.bottom() {
            for x in sel_rect.left()..sel_rect.right() {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    let fg = cell.fg;
                    let bg = cell.bg;
                    if fg == bg {
                        // Swapping identical colors is invisible
                        // (e.g. both Reset — the default for user messages
                        // and empty cells). Use explicit highlight colors.
                        cell.set_fg(Color::Black);
                        cell.set_bg(Color::White);
                    } else {
                        cell.set_fg(bg);
                        cell.set_bg(fg);
                    }
                }
            }
        }
    }
}

/// If a clipboard copy is pending, extracts the selected text from the buffer
/// and copies it to the system clipboard. Clears the pending flag regardless
/// of success or failure.
///
/// The clipboard write runs on a spawned thread that holds the
/// [`arboard::Clipboard`] open for a few seconds after writing. On X11,
/// clipboard data is only available while the `Clipboard` instance is alive —
/// dropping it immediately prevents clipboard managers from syncing.
fn flush_pending_clipboard(app: &mut TuiApp, buf: &ratatui::buffer::Buffer) {
    if !app.pending_clipboard {
        return;
    }
    app.pending_clipboard = false;

    let text = match app.selection.extract_text(buf) {
        Some(text) if !text.is_empty() => text,
        _ => return,
    };

    // Spawn a thread to hold the clipboard open for clipboard managers.
    std::thread::spawn(move || {
        let mut cb = match arboard::Clipboard::new() {
            Ok(cb) => cb,
            Err(e) => {
                tracing::warn!(err = %e, "failed to create clipboard");
                return;
            }
        };
        if let Err(e) = cb.set_text(&text) {
            tracing::warn!(err = %e, "failed to copy selection to clipboard");
            return;
        }
        tracing::debug!(len = text.len(), "copied selection to clipboard");
        // Hold clipboard open so clipboard managers can sync.
        // cb must live through the sleep — X11 clipboard data is only
        // available while the Clipboard instance is alive.
        std::thread::sleep(std::time::Duration::from_secs(2));
    });
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
fn render_too_small(frame: &mut Frame<'_>, area: Rect, app: &mut TuiApp) {
    let msg = format!("Terminal too small\n{MIN_WIDTH}x{MIN_HEIGHT} minimum");
    let paragraph = Paragraph::new(msg).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
    // Clear selectable rects when terminal is too small.
    app.selectable_rects.rebuild(vec![]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::selection::SelectionState;
    use nullslop_selection_widget::compute_popup_rect;
    use ratatui::style::Modifier;

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

    // --- Selection highlight tests ---

    #[test]
    fn selection_highlight_inverts_cells_within_selection() {
        // Given a buffer with distinctively colored cells and an active selection.
        let area = Rect::new(0, 0, 20, 10);
        let mut buf = ratatui::buffer::Buffer::empty(area);
        // Paint a cell inside the selection with known colors.
        buf.cell_mut((3, 3)).unwrap().set_fg(Color::Yellow);
        buf.cell_mut((3, 3)).unwrap().set_bg(Color::Blue);
        // Paint a cell outside the selection with known colors.
        buf.cell_mut((15, 8)).unwrap().set_fg(Color::Red);
        buf.cell_mut((15, 8)).unwrap().set_bg(Color::Green);

        // And an app with an Active selection covering (2,2) to (5,4).
        let services = nullslop_services::test_services::TestServices::builder().build();
        let mut app = crate::TuiApp::new(services);
        app.selection = SelectionState::Active {
            anchor: (2, 2),
            focus: (5, 4),
            bounds: area,
        };

        // When applying selection highlight.
        apply_selection_highlight(&app, &mut buf);

        // Then cell (3, 3) inside the selection has swapped fg/bg.
        let inside = buf.cell((3, 3)).expect("cell inside selection");
        assert_eq!(inside.fg, Color::Blue); // was bg
        assert_eq!(inside.bg, Color::Yellow); // was fg

        // And cell (15, 8) outside the selection is unchanged.
        let outside = buf.cell((15, 8)).expect("cell outside selection");
        assert_eq!(outside.fg, Color::Red);
        assert_eq!(outside.bg, Color::Green);
    }

    #[test]
    fn selection_highlight_respects_constraining_bounds() {
        // Given a buffer covering a large area and a selection where the raw anchor
        // extends beyond the selection's constraining bounds.
        let full_area = Rect::new(0, 0, 30, 30);
        let mut buf = ratatui::buffer::Buffer::empty(full_area);
        // Paint cell inside bounds (will be in clamped selection).
        buf.cell_mut((7, 7)).unwrap().set_fg(Color::Cyan);
        buf.cell_mut((7, 7)).unwrap().set_bg(Color::Magenta);
        // Paint cell at raw anchor position (0, 0) — outside bounds.
        buf.cell_mut((0, 0)).unwrap().set_fg(Color::White);
        buf.cell_mut((0, 0)).unwrap().set_bg(Color::Black);

        // And an Active selection with anchor outside bounds.
        // bounds=(5,5,10,10) means valid range is (5,5)-(14,14).
        // anchor=(0,0) is outside bounds, focus=(8,8) is inside.
        // selection_rect() should clamp to (5,5)-(8,8).
        let bounds = Rect::new(5, 5, 10, 10);
        let services = nullslop_services::test_services::TestServices::builder().build();
        let mut app = crate::TuiApp::new(services);
        app.selection = SelectionState::Active {
            anchor: (0, 0),
            focus: (8, 8),
            bounds,
        };

        // When applying selection highlight.
        apply_selection_highlight(&app, &mut buf);

        // Then cell (7, 7) inside the clamped selection is inverted.
        let inside = buf.cell((7, 7)).expect("cell inside clamped selection");
        assert_eq!(inside.fg, Color::Magenta); // was bg
        assert_eq!(inside.bg, Color::Cyan); // was fg

        // And cell (0, 0) at the raw anchor position is NOT inverted.
        let outside = buf.cell((0, 0)).expect("cell at raw anchor");
        assert_eq!(outside.fg, Color::White); // unchanged
        assert_eq!(outside.bg, Color::Black); // unchanged
    }

    #[test]
    fn selection_highlight_does_nothing_when_idle() {
        // Given a buffer with distinctly colored cells and an Idle selection.
        let area = Rect::new(0, 0, 20, 10);
        let mut buf = ratatui::buffer::Buffer::empty(area);
        buf.cell_mut((5, 5)).unwrap().set_fg(Color::Yellow);
        buf.cell_mut((5, 5)).unwrap().set_bg(Color::Blue);

        // And an app with an Idle selection.
        let services = nullslop_services::test_services::TestServices::builder().build();
        let mut app = crate::TuiApp::new(services);
        app.selection = SelectionState::Idle;

        // When applying selection highlight.
        apply_selection_highlight(&app, &mut buf);

        // Then no cells are inverted — colors remain unchanged.
        let cell = buf.cell((5, 5)).expect("colored cell");
        assert_eq!(cell.fg, Color::Yellow); // unchanged
        assert_eq!(cell.bg, Color::Blue); // unchanged
    }

    #[test]
    fn selection_highlight_uses_explicit_colors_when_fg_equals_bg() {
        // Given a buffer where cells have matching fg and bg (e.g. both Reset,
        // as with user messages rendered with Style::default().bold()).
        let area = Rect::new(0, 0, 20, 10);
        let mut buf = ratatui::buffer::Buffer::empty(area);
        // User-message-style cell: fg = Reset, bg = Reset (bold modifier).
        buf.cell_mut((3, 3))
            .unwrap()
            .set_style(Style::default().add_modifier(Modifier::BOLD));
        // Adjacent cell with distinct colors (assistant-style).
        buf.cell_mut((4, 3)).unwrap().set_fg(Color::Cyan);

        // And an Active selection covering both cells.
        let services = nullslop_services::test_services::TestServices::builder().build();
        let mut app = crate::TuiApp::new(services);
        app.selection = SelectionState::Active {
            anchor: (2, 2),
            focus: (5, 4),
            bounds: area,
        };

        // When applying selection highlight.
        apply_selection_highlight(&app, &mut buf);

        // Then the Reset/Reset cell gets explicit highlight colors.
        let reset_cell = buf.cell((3, 3)).expect("reset cell");
        assert_eq!(reset_cell.fg, Color::Black);
        assert_eq!(reset_cell.bg, Color::White);

        // And the distinct-colors cell gets swapped fg/bg.
        let cyan_cell = buf.cell((4, 3)).expect("cyan cell");
        assert_eq!(cyan_cell.fg, Color::Reset); // was bg
        assert_eq!(cyan_cell.bg, Color::Cyan); // was fg
    }

    // --- Clipboard flush tests ---

    #[test]
    fn clipboard_copy_clears_pending_flag_on_idle_selection() {
        // Given an app with pending_clipboard set but Idle selection.
        let services = nullslop_services::test_services::TestServices::builder().build();
        let mut app = crate::TuiApp::new(services);
        app.selection = SelectionState::Idle;
        app.pending_clipboard = true;

        let area = Rect::new(0, 0, 20, 5);
        let buf = ratatui::buffer::Buffer::empty(area);

        // When flushing the pending clipboard.
        flush_pending_clipboard(&mut app, &buf);

        // Then the pending flag is cleared (even though there was nothing to copy).
        assert!(!app.pending_clipboard);
    }

    #[test]
    fn clipboard_copy_skips_empty_selection() {
        // Given an app with pending_clipboard and an Active selection over empty cells.
        let area = Rect::new(0, 0, 20, 5);
        let buf = ratatui::buffer::Buffer::empty(area);

        let services = nullslop_services::test_services::TestServices::builder().build();
        let mut app = crate::TuiApp::new(services);
        app.selection = SelectionState::Active {
            anchor: (0, 0),
            focus: (3, 0),
            bounds: area,
        };
        app.pending_clipboard = true;

        // When flushing the pending clipboard.
        flush_pending_clipboard(&mut app, &buf);

        // Then the pending flag is cleared.
        assert!(!app.pending_clipboard);
    }

    #[test]
    #[ignore = "requires clipboard access (run with --ignored)"]
    fn clipboard_copy_extracts_selected_text() {
        // Given a buffer with known text and an active selection.
        let area = Rect::new(0, 0, 20, 5);
        let mut buf = ratatui::buffer::Buffer::empty(area);
        // Write "Hello" on row 2.
        for (i, ch) in "Hello".chars().enumerate() {
            buf.cell_mut((2 + i as u16, 2))
                .unwrap()
                .set_symbol(&ch.to_string());
        }

        let services = nullslop_services::test_services::TestServices::builder().build();
        let mut app = crate::TuiApp::new(services);
        app.selection = SelectionState::Active {
            anchor: (2, 2),
            focus: (6, 2),
            bounds: area,
        };
        app.pending_clipboard = true;

        // When flushing the pending clipboard.
        flush_pending_clipboard(&mut app, &buf);

        // Then the pending flag is cleared immediately.
        assert!(!app.pending_clipboard);

        // And after the clipboard thread completes, the clipboard contains
        // the selected text.
        std::thread::sleep(std::time::Duration::from_millis(500));
        let mut clipboard = arboard::Clipboard::new().expect("clipboard access");
        let content = clipboard.get_text().expect("read clipboard");
        assert_eq!(content, "Hello");
    }

    // --- Element-driven selectable rect tests ---

    #[test]
    fn render_registers_content_rect_for_selectable_chat_log() {
        // Given a TuiApp rendered in Chat tab with a 80x24 terminal.
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let services = nullslop_services::test_services::TestServices::builder().build();
        let mut app = crate::TuiApp::new(services);
        // Default tab is Chat.

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // When rendering.
        terminal
            .draw(|frame| {
                app.render(frame);
            })
            .unwrap();

        // Then the content area rect is registered as selectable.
        // Chat log is selectable, so layout.content should be in selectable_rects.
        let layout = AppLayout::new(frame_area(80, 24), 1, 0);
        let found = app.selectable_rects.find_for_position(
            layout.content.x + 1,
            layout.content.y + 1,
        );
        assert!(
            found.is_some(),
            "chat log content rect should be selectable"
        );
        assert_eq!(found.unwrap(), layout.content);
    }

    #[test]
    fn render_registers_picker_popup_rect_when_active() {
        // Given a TuiApp rendered with Mode::Picker.
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let services = nullslop_services::test_services::TestServices::builder().build();
        let mut app = crate::TuiApp::new(services);
        // Switch to Picker mode.
        app.core.state.write().mode = nullslop_protocol::Mode::Picker;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // When rendering.
        terminal
            .draw(|frame| {
                app.render(frame);
            })
            .unwrap();

        // Then the picker popup rect is registered as selectable.
        let popup_rect = compute_popup_rect(Rect::new(0, 0, 80, 24));
        let found = app
            .selectable_rects
            .find_for_position(popup_rect.x + 1, popup_rect.y + 1);
        assert!(
            found.is_some(),
            "picker popup rect should be selectable"
        );
        assert_eq!(found.unwrap(), popup_rect);

        // And the content area rect is also still selectable (chat-log is selectable).
        let layout = AppLayout::new(frame_area(80, 24), 1, 0);
        let content_found = app.selectable_rects.find_for_position(
            layout.content.x + 1,
            layout.content.y + 1,
        );
        assert!(
            content_found.is_some(),
            "content rect should also be selectable alongside picker"
        );
    }

    /// Helper to create a Rect matching the terminal dimensions.
    fn frame_area(w: u16, h: u16) -> Rect {
        Rect::new(0, 0, w, h)
    }
}
