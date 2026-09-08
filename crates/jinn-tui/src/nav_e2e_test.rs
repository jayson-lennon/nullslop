//! E2E: j-keypress → keymap → router → route row → DashboardNav → actor → cell.
#![allow(clippy::expect_used, reason = "test code")]
use jinn_domain::feat::dashboard::DashboardState;
use jinn_domain::feat::dashboard::dashboard_slot;

#[rstest::rstest]
#[tokio::test]
async fn j_keypress_routes_to_dashboard_actor_and_moves_selection() {
    // Given a wired app: the test builder runs `dashboard::activate`,
    // which spawns THE dashboard actor subscribed to the bus.
    let mut app = crate::TuiApp::test_builder().build().await;
    app.core
        .state
        .write_test_no_cap()
        .frontend
        .scope_stack
        .swap_base(jinn_domain::FocusScope::Dynamic(
            jinn_domain::feat::dashboard::dashboard_scope(),
        ));
    let slot = dashboard_slot();
    let cell: jinn_domain::common::slices::TypedCell<DashboardState> =
        app.services.slices.reader(&slot).expect("cell");
    cell.update(|d| {
        for i in 0..3 {
            d.mark_running(format!("actor-{i}"), None);
        }
    });

    // When the j key resolves through the keymap and routes like the run loop.
    app.which_key.set_scope(crate::scope::Scope::Dynamic(
        jinn_domain::feat::dashboard::dashboard_scope(),
    ));
    let protocol_key = {
        use crossterm::event::{KeyCode, KeyEvent as XKeyEvent, KeyModifiers};
        crate::convert::from_crossterm(XKeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE))
            .expect("j converts")
    };
    let before = cell.read().selected_index();
    let intent = app
        .which_key
        .handle_key(protocol_key)
        .expect("j resolves in Dashboard scope");
    app.route_intent(intent);
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Then the actor applied the move to the slice.
    let after = cell.read().selected_index();
    assert_eq!(
        after,
        before + 1,
        "j moves selection via the routed message"
    );
}
