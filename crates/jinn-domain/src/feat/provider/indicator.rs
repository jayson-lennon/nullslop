//! Streaming indicator element with animated throbber.
//!
//! Renders an animated ASCII spinner alongside "Working..." when the active
//! session is busy (sending, streaming, or compacting), and renders nothing
//! when idle. Queue count is shown when messages are waiting (not during
//! compaction).

use std::time::{Duration, Instant};

use crate::common::render_ctx::RenderCtx;
use crate::common::ui_element::UiElement;
use crate::feat::session::phase_machine::PhaseKind;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use throbber_widgets_tui::{Throbber, ThrobberState, WhichUse};

/// Minimum time between animation frame advances.
const ANIMATION_INTERVAL: Duration = Duration::from_millis(80);

/// Displays an animated streaming indicator when the active session is sending, streaming, or compacting.
#[derive(Debug)]
pub struct StreamingIndicatorElement {
    /// Visual-only state for the throbber animation step.
    throbber_state: ThrobberState,
    /// Timestamp of the last animation frame advance.
    last_animation_step: Instant,
}

impl StreamingIndicatorElement {
    /// Creates a new streaming indicator element.
    pub fn new() -> Self {
        Self {
            throbber_state: ThrobberState::default(),
            last_animation_step: Instant::now(),
        }
    }

    /// Advances the animation frame if enough time has elapsed.
    fn maybe_advance_animation(&mut self) {
        if self.last_animation_step.elapsed() >= ANIMATION_INTERVAL {
            self.throbber_state.calc_next();
            self.last_animation_step = Instant::now();
        }
    }
}

impl Default for StreamingIndicatorElement {
    fn default() -> Self {
        Self::new()
    }
}

impl UiElement for StreamingIndicatorElement {
    fn name(&self) -> String {
        "streaming-indicator".to_owned()
    }

    fn render(&mut self, frame: &mut Frame<'_>, area: Rect, ctx: &RenderCtx) {
        let state = ctx.state;
        let session = state.active_session();
        let phase = session.phase();

        let is_busy = session.is_busy();
        let is_phase_busy = matches!(phase, PhaseKind::Sending | PhaseKind::Streaming);

        if !is_busy && !is_phase_busy {
            return;
        }

        let label = if is_busy {
            " Working..."
        } else {
            " Streaming..."
        };

        let throbber = Throbber::default()
            .label(label)
            .style(Style::default().fg(state.frontend.theme.streaming))
            .throbber_style(Style::default().fg(state.frontend.theme.streaming))
            .throbber_set(throbber_widgets_tui::ASCII)
            .use_type(WhichUse::Spin);

        frame.render_stateful_widget(throbber, area, &mut self.throbber_state);

        // Advance the animation step only when enough time has elapsed.
        self.maybe_advance_animation();
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable,
        clippy::indexing_slicing,
        reason = "test code"
    )]
    use crate::AppState;

    use super::*;

    #[rstest::rstest]
    fn name_returns_streaming_indicator() {
        // Given a StreamingIndicatorElement.
        let element = StreamingIndicatorElement::new();

        // When querying the name.
        let name = element.name();

        // Then it is "streaming-indicator".
        assert_eq!(name, "streaming-indicator");
    }

    #[rstest::rstest]
    fn renders_streaming_label_during_sending_phase() {
        // Given a session in Sending phase.
        use jinn_testutil::{buffer_row, setup_term};

        let mut element = StreamingIndicatorElement::new();
        let mut state = AppState::default();
        state.active_session_mut().begin_sending();
        let (mut terminal, area) = setup_term(30, 1);
        terminal
            .draw(|frame| {
                let slices = jinn_slices::Slices::new();
                let ctx = RenderCtx::new(&state, &slices);
                element.render(frame, area, &ctx);
            })
            .unwrap();
        let buffer = terminal.backend().buffer().clone();
        let row = buffer_row(&buffer, 0, 30);

        // Then the label shows "Streaming...".
        assert!(
            row.contains("Streaming..."),
            "expected Streaming..., got: {row}"
        );
    }

    #[rstest::rstest]
    fn does_not_render_during_idle_phase() {
        // Given a session in Idle phase (default).
        use jinn_testutil::setup_term;

        let mut element = StreamingIndicatorElement::new();
        let state = AppState::default();
        let (mut terminal, area) = setup_term(30, 1);
        terminal
            .draw(|frame| {
                let slices = jinn_slices::Slices::new();
                let ctx = RenderCtx::new(&state, &slices);
                element.render(frame, area, &ctx);
            })
            .unwrap();
        let buffer = terminal.backend().buffer().clone();

        // Then the rendered area is empty (all spaces).
        let content: String = (0..30)
            .filter_map(|x| buffer.cell((x, 0)).map(ratatui::buffer::Cell::symbol))
            .collect();
        assert!(
            content.trim().is_empty(),
            "expected empty buffer, got: {content}"
        );
    }

    #[rstest::rstest]
    fn renders_working_for_marking_busy_when_idle() {
        // Given a session with busy counter set but phase Idle.
        use jinn_testutil::{buffer_row, setup_term};

        let mut element = StreamingIndicatorElement::new();
        let mut state = AppState::default();
        state.active_session_mut().begin_busy();
        let (mut terminal, area) = setup_term(30, 1);
        terminal
            .draw(|frame| {
                let slices = jinn_slices::Slices::new();
                let ctx = RenderCtx::new(&state, &slices);
                element.render(frame, area, &ctx);
            })
            .unwrap();
        let buffer = terminal.backend().buffer().clone();
        let row = buffer_row(&buffer, 0, 30);

        // Then the label shows "Working...".
        assert!(
            row.contains("Working..."),
            "expected Working..., got: {row}"
        );
    }
}
