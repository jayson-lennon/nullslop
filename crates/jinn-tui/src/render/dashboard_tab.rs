//! Dashboard tab rendering — resolves the dashboard slice through the
//! [`Slices`] registry and draws it with [`DashboardView`].
//!
//! This module is now a thin adapter: the table-drawing logic lives in the
//! dashboard feature's VIEW artifact ([`DashboardView`]), which renders
//! from the slice payload directly. The AppState-based renderer was removed
//! with the `frontend.dashboard` field.

use jinn_domain::RenderCtx;
use jinn_domain::common::slices::SliceView;
use jinn_domain::common::slices::Slices;
use jinn_domain::common::slices::ViewCx;
use jinn_domain::feat::dashboard::DashboardState;
use jinn_domain::feat::dashboard::DashboardView;
use jinn_domain::feat::dashboard::dashboard_slot;
use ratatui::Frame;
use ratatui::layout::Rect;

/// Renders the full dashboard view into `area` (the content rect of the tab)
/// by resolving the dashboard slice through the app's slice registry.
pub fn render_dashboard(frame: &mut Frame<'_>, area: Rect, ctx: &RenderCtx, slices: &Slices) {
    let theme = &ctx.state.frontend.theme;
    let cx = ViewCx { theme };

    let cell = resolve_slice(slices);
    let guard = cell.read();
    let mut view = DashboardView::new();
    view.render(frame, area, &cx, &guard);
}

/// Resolves the dashboard slice cell from the registry.
///
/// The slot is registered at startup before the first frame (the actor
/// mint at spawn), so the fallback is unreachable in a wired app; a
/// detached scratch cell keeps renders safe if a wiring regression lands.
#[expect(
    clippy::unreachable,
    reason = "the Err arm is unreachable for a fresh registry; a hit means a bug in Slices"
)]
fn resolve_slice(
    slices: &Slices,
) -> jinn_domain::common::slices::TypedCell<jinn_domain::feat::dashboard::DashboardState> {
    if let Some(cell) = slices.reader(&dashboard_slot()) {
        return cell;
    }
    let detached = jinn_domain::common::slices::Slices::new();
    // A fresh registry cannot conflict; `Err` is unreachable.
    match detached.register(dashboard_slot(), DashboardState::default()) {
        Ok(cell) => cell,
        Err(error) => unreachable!("fresh registry cannot conflict: {error}"),
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        clippy::unreachable,
        clippy::indexing_slicing,
        reason = "test code"
    )]
    use jinn_domain::feat::dashboard::DashboardState;
    use jinn_domain::feat::dashboard::dashboard_slot;
    use jinn_testutil::setup_term;

    async fn build_app() -> crate::TuiApp {
        crate::TuiApp::test_builder().build().await
    }

    /// Resolves the dashboard cell from the app's slice registry.
    fn dashboard_cell(
        app: &crate::TuiApp,
    ) -> jinn_domain::common::slices::TypedCell<DashboardState> {
        app.services
            .slices
            .reader(&dashboard_slot())
            .expect("test builder registers the dashboard slot")
    }

    fn write_dashboard(app: &crate::TuiApp, f: impl FnOnce(&mut DashboardState)) {
        dashboard_cell(app).update(f);
    }

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
    #[tokio::test]
    async fn renders_actor_name_and_lifecycle() {
        // Given a dashboard with one running actor.
        let mut app = build_app().await;
        app.core
            .state
            .write_test_no_cap()
            .frontend
            .scope_stack
            .swap_base(jinn_domain::FocusScope::Dashboard);
        write_dashboard(&app, |d| {
            d.mark_running("discord", Some("Discord bot".to_owned()));
        });
        let (mut terminal, _area) = setup_term(80, 24);

        // When rendering.
        terminal.draw(|frame| app.render(frame)).unwrap();

        // Then the buffer contains "discord" and "Running".
        let buf_str = buffer_string(&terminal);
        assert!(buf_str.contains("discord"), "dashboard should show name");
        assert!(
            buf_str.contains("Running"),
            "dashboard should show lifecycle"
        );
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn renders_status_message_for_discord() {
        // Given a dashboard with discord in a connected state.
        let mut app = build_app().await;
        app.core
            .state
            .write_test_no_cap()
            .frontend
            .scope_stack
            .swap_base(jinn_domain::FocusScope::Dashboard);
        write_dashboard(&app, |d| {
            d.mark_running("discord", None);
            d.set_status_message("discord", Some("Connected".to_owned()));
        });
        let (mut terminal, _area) = setup_term(80, 24);

        // When rendering.
        terminal.draw(|frame| app.render(frame)).unwrap();

        // Then the buffer contains "Connected".
        let buf_str = buffer_string(&terminal);
        assert!(
            buf_str.contains("Connected"),
            "dashboard should show status message"
        );
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn renders_empty_placeholder_when_no_actors() {
        // Given a dashboard cell with no actor rows.
        let mut app = build_app().await;
        app.core
            .state
            .write_test_no_cap()
            .frontend
            .scope_stack
            .swap_base(jinn_domain::FocusScope::Dashboard);
        write_dashboard(&app, jinn_domain::feat::dashboard::DashboardState::clear);
        let (mut terminal, _area) = setup_term(80, 24);

        // When rendering.
        terminal.draw(|frame| app.render(frame)).unwrap();

        // Then the buffer contains the placeholder.
        let buf_str = buffer_string(&terminal);
        assert!(
            buf_str.contains("No services"),
            "empty dashboard should show placeholder"
        );
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn shows_selection_marker_on_selected_entry() {
        // Given a dashboard with two actors, second selected.
        let mut app = build_app().await;
        app.core
            .state
            .write_test_no_cap()
            .frontend
            .scope_stack
            .swap_base(jinn_domain::FocusScope::Dashboard);
        write_dashboard(&app, |d| {
            d.mark_running("alpha", None);
            d.mark_running("beta", None);
            d.select_next(); // select beta (index 1)
        });
        let (mut terminal, _area) = setup_term(80, 24);

        // When rendering.
        terminal.draw(|frame| app.render(frame)).unwrap();

        // Then the buffer contains the selection marker ▸.
        let buf_str = buffer_string(&terminal);
        assert!(buf_str.contains('▸'), "selected entry should have marker");
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn renders_no_em_dash_separator() {
        // Given a dashboard with an actor that has a description.
        let mut app = build_app().await;
        app.core
            .state
            .write_test_no_cap()
            .frontend
            .scope_stack
            .swap_base(jinn_domain::FocusScope::Dashboard);
        write_dashboard(&app, |d| {
            d.mark_running("discord", Some("Discord bot".to_owned()));
        });
        let (mut terminal, _area) = setup_term(80, 24);

        // When rendering.
        terminal.draw(|frame| app.render(frame)).unwrap();

        // Then the buffer contains no em-dash characters.
        let buf_str = buffer_string(&terminal);
        assert!(
            !buf_str.contains('\u{2014}'),
            "dashboard should not contain em-dashes"
        );
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
