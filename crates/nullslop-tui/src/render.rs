//! Layout computation and rendering for the application.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui_tabs::{TabManager, TabsBar, TabsStyle};
use ratatui_which_key::{PopupPosition, WhichKey};

use crate::TuiApp;

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
    let layout = AppLayout::new(area, state.active_chat_input().visual_line_count() as u16, queue_len);

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
        .max_height(10)
        .position(PopupPosition::BottomRight)
        .border_style(Style::default().fg(Color::Yellow));
    let buf = frame.buffer_mut();
    widget.render(buf, state);
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
}
