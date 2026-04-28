//! Layout computation and rendering for the application.

use ratatui::style::{Color, Style};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui_which_key::{PopupPosition, WhichKey};

use crate::TuiApp;

/// Minimum terminal width.
pub const MIN_WIDTH: u16 = 40;
/// Minimum terminal height.
pub const MIN_HEIGHT: u16 = 12;

/// Top-level application layout areas.
pub struct AppLayout {
    /// The chat log area (fills remaining space).
    pub chat: Rect,
    /// The input box area (3 rows at bottom).
    pub input: Rect,
}

impl AppLayout {
    /// Returns `true` if the given area meets minimum size requirements.
    #[must_use]
    pub const fn meets_min_size(area: Rect) -> bool {
        area.width >= MIN_WIDTH && area.height >= MIN_HEIGHT
    }

    /// Computes the layout for the given terminal area.
    #[must_use]
    pub fn new(area: Rect) -> Self {
        let [chat, input] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(3)]).areas(area);

        Self { chat, input }
    }
}

/// Renders the full application frame.
pub fn render(app: &mut TuiApp, frame: &mut Frame<'_>) {
    let area = frame.area();
    if !AppLayout::meets_min_size(area) {
        render_too_small(frame, area);
        return;
    }

    let layout = AppLayout::new(area);
    let state = app.core.state.read();

    // Chat log — delegate to registry element
    if let Some(element) = app.ui_registry.get_mut("chat-log") {
        element.render(frame, layout.chat, &state);
    }

    // Input box — delegate to registry element
    if let Some(element) = app.ui_registry.get_mut("chat-input-box") {
        element.render(frame, layout.input, &state);
    }

    // Which-key popup overlay (app-level, not a plugin element)
    render_which_key(frame, &mut app.which_key);
}

/// Renders the which-key popup overlay.
fn render_which_key(frame: &mut Frame<'_>, state: &mut crate::app::WhichKeyInstance) {
    let widget = WhichKey::new()
        .max_height(10)
        .position(PopupPosition::BottomRight)
        .border_style(Style::default().fg(Color::Green));
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
        // Given a 40x12 area.
        let area = Rect::new(0, 0, 40, 12);

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
    fn app_layout_splits() {
        // Given an 80x24 area.
        let area = Rect::new(0, 0, 80, 24);

        // When computing layout.
        let layout = AppLayout::new(area);

        // Then chat height is 21 and input height is 3.
        assert_eq!(layout.chat.height, 21);
        assert_eq!(layout.input.height, 3);
    }
}
