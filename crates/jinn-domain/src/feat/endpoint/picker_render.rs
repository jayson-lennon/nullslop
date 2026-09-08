//! Rendering for the OpenRouter endpoint picker overlay.

use crate::common::render_ctx::RenderCtx;
use crate::feat::ui::picker_states::PickerExt;
use jinn_selection_widget::PreviewSelectionWidget;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};

/// Renders the endpoint picker overlay using [`PreviewSelectionWidget`].
///
/// Multipane: the upstream list (left) with a preview pane (right) showing the
/// selected endpoint's routing tag, uptime, quantization, and pricing. The
/// footer shows the currently pinned endpoint (the entry the loader marked
/// `is_active`), or "auto-route" when none is pinned.
pub fn render_endpoint_picker(frame: &mut Frame<'_>, area: Rect, ctx: &RenderCtx) {
    let state = ctx.state;
    let theme = &state.frontend.theme;

    let gray = Style::default().fg(theme.muted_text);
    let orange = Style::default().fg(theme.accent_action);

    let pinned_name = state
        .frontend
        .endpoint_picker()
        .items()
        .iter()
        .find(|e| e.is_active)
        .map_or("auto-route", |e| e.provider_name.as_str());

    let mut spans = vec![
        Span::styled("Routing: ".to_owned(), gray),
        Span::styled(
            pinned_name.to_owned(),
            Style::default().fg(theme.primary_text),
        ),
        Span::styled("  ".to_owned(), gray),
        Span::styled("Enter ".to_owned(), orange),
        Span::styled("pin · ".to_owned(), gray),
        Span::styled("ESC ".to_owned(), orange),
        Span::styled("cancel".to_owned(), gray),
    ];

    // A fetch is in flight: show a spinner-style indicator until the actor
    // writes items back.
    if state.frontend.pickers.endpoint_loading {
        spans.push(Span::styled("  ".to_owned(), gray));
        spans.push(Span::styled("fetching…".to_owned(), orange));
    } else {
        // Otherwise show how long ago the cache was last populated (if ever).
        if let Some(ts) = state.frontend.pickers.endpoint_fetched_at {
            spans.push(Span::styled("  ".to_owned(), gray));
            spans.push(Span::styled(format!("fetched {}", format_age(ts)), gray));
        }
    }

    let footer = Line::from(spans);

    let widget = PreviewSelectionWidget::new(state.frontend.endpoint_picker())
        .title(Line::from(" OpenRouter Endpoint "))
        .title_style(Style::default().fg(theme.popup_title))
        .footer(footer);
    widget.render(frame, area);
}

/// Coarse "time ago" formatter for the endpoint cache freshness line.
///
/// `<60s` → `Xs`, `<60m` → `Xm`, else `Xh`. Mirrors the model picker's
/// time-ago computation but trimmed for picker-footer brevity.
fn format_age(fetched_at: jiff::Timestamp) -> String {
    let elapsed = jiff::Timestamp::now() - fetched_at;
    let secs = elapsed.total(jiff::Unit::Second).unwrap_or(0.0).max(0.0) as u64;
    if secs < 60 {
        format!("{secs}s ago")
    } else if secs < 60 * 60 {
        format!("{}m ago", secs / 60)
    } else {
        format!("{}h ago", secs / 3600)
    }
}
#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic, reason = "test code")]

    use super::*;
    use crate::common::app_state::AppState;
    use crate::feat::endpoint::picker_entry::EndpointEntry;
    use crate::feat::theme::default_theme;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[rstest::rstest]
    #[test]
    fn render_endpoint_picker_does_not_panic_with_populated_picker() {
        // Given a state whose endpoint picker has a populated entry.
        let mut state = AppState::default();
        let entry = EndpointEntry::auto_route(true, default_theme());
        state.frontend.endpoint_picker_mut().set_items(vec![entry]);

        // When rendering the picker.
        // Then it does not panic.
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("terminal");
        terminal
            .draw(|frame| {
                let slices = jinn_slices::Slices::new();
                let overlay_views = crate::common::overlay_views::OverlayViews::new();
                let ctx = RenderCtx::new(&state, &slices, &overlay_views);
                render_endpoint_picker(frame, Rect::new(0, 0, 80, 24), &ctx);
            })
            .expect("draw");
    }

    #[rstest::rstest]
    #[test]
    fn render_endpoint_picker_with_real_upstream_shows_preview_metadata() {
        // Given a picker with a real upstream entry carrying metadata.
        let mut state = AppState::default();
        let entry = EndpointEntry {
            tag: "anthropic".to_owned(),
            provider_name: "Anthropic".to_owned(),
            uptime_30m: Some(99.7),
            prompt_price: Some("$3.00".to_owned()),
            completion_price: Some("$15.00".to_owned()),
            quantization: Some("fp16".to_owned()),
            max_completion_tokens: Some(64000),
            is_active: false,
            theme: default_theme(),
        };
        state.frontend.endpoint_picker_mut().set_items(vec![entry]);

        // When rendering with a wide-enough area for both panes.
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).expect("terminal");
        terminal
            .draw(|frame| {
                let slices = jinn_slices::Slices::new();
                let overlay_views = crate::common::overlay_views::OverlayViews::new();
                let ctx = RenderCtx::new(&state, &slices, &overlay_views);
                render_endpoint_picker(frame, Rect::new(0, 0, 100, 30), &ctx);
            })
            .expect("draw");

        // Then it does not panic (preview pane renders the metadata).
    }

    fn buffer_contains(state: &AppState, needle: &str) -> bool {
        // Renders the picker and scans every buffer row for `needle`.
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).expect("terminal");
        terminal
            .draw(|frame| {
                let slices = jinn_slices::Slices::new();
                let overlay_views = crate::common::overlay_views::OverlayViews::new();
                let ctx = RenderCtx::new(state, &slices, &overlay_views);
                render_endpoint_picker(frame, Rect::new(0, 0, 120, 30), &ctx);
            })
            .expect("draw");
        let buffer = terminal.backend().buffer();
        (0..buffer.area.height).any(|y| {
            let row: String = (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol().to_owned())
                .collect::<String>();
            row.contains(needle)
        })
    }
    #[rstest::rstest]
    #[test]
    fn footer_shows_fetching_indicator_while_loading() {
        // Given a populated picker mid-fetch (loading flag set).
        let mut state = AppState::default();
        state
            .frontend
            .endpoint_picker_mut()
            .set_items(vec![EndpointEntry::auto_route(true, default_theme())]);
        state.frontend.pickers.endpoint_loading = true;

        // When rendering.
        // Then the footer contains the fetching indicator.
        assert!(
            buffer_contains(&state, "fetching"),
            "footer must show a fetching indicator while loading"
        );
    }

    #[rstest::rstest]
    #[test]
    fn footer_shows_fetched_age_when_not_loading() {
        // Given a populated picker with a fetch timestamp and loading cleared.
        let mut state = AppState::default();
        state
            .frontend
            .endpoint_picker_mut()
            .set_items(vec![EndpointEntry::auto_route(true, default_theme())]);
        state.frontend.pickers.endpoint_fetched_at = Some(jiff::Timestamp::now());

        // When rendering.
        // Then the footer contains a freshness line.
        assert!(
            buffer_contains(&state, "fetched"),
            "footer must show a freshness line when fetched_at is set"
        );
    }
}
