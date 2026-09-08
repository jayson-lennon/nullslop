//! Dashboard keybind routing — the feature's contribution to [`KeyRoutes`].
//!
//! The four navigation intents are no longer handled by arms in the central
//! intent handler: this module registers route rows mapping each intent to a
//! [`DashboardNav`] message for the dashboard actor. The router consults the
//! table (via `KeyRoutes::action_for`) and emits the message onto the bus;
//! the dashboard actor — the slice's single writer — updates the cell.
//!
//! Registering rows here (rather than hard-coding arms) is what makes the
//! pattern extensible: a guest plugin attaches its own rows against the
//! same table, addressing the plugin coordinator.

use crate::common::slices::key_routes::KeyRoutes;
use crate::protocol::IntentResult;
use crate::protocol::intent::Intent;

use super::DashboardNav;

/// Registers the dashboard feature's keybind rows onto the shared route
/// table. Called once at bootstrap (see the TUI launch wiring).
pub fn attach_dashboard_rows(routes: &KeyRoutes) {
    routes.attach_builtin(
        &Intent::DashboardSelectUp,
        || IntentResult::new_message(DashboardNav::Up),
        "dashboard",
    );
    routes.attach_builtin(
        &Intent::DashboardSelectDown,
        || IntentResult::new_message(DashboardNav::Down),
        "dashboard",
    );
    routes.attach_builtin(
        &Intent::DashboardSelectFirst,
        || IntentResult::new_message(DashboardNav::First),
        "dashboard",
    );
    routes.attach_builtin(
        &Intent::DashboardSelectLast,
        || IntentResult::new_message(DashboardNav::Last),
        "dashboard",
    );
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        reason = "test code; a missing attached row is a hard failure"
    )]
    use super::attach_dashboard_rows;
    use crate::common::slices::key_routes::KeyRoutes;
    use crate::protocol::intent::Intent;

    #[rstest::rstest]
    #[case::up(Intent::DashboardSelectUp)]
    #[case::down(Intent::DashboardSelectDown)]
    #[case::first(Intent::DashboardSelectFirst)]
    #[case::last(Intent::DashboardSelectLast)]
    fn route_lookup_produces_nav_message_for_bound_intent(#[case] intent: Intent) {
        // Given a route table with the dashboard rows attached.
        let routes = KeyRoutes::new();
        attach_dashboard_rows(&routes);

        // When looking up the action for a bound intent.
        let result = routes.action_for(&intent).expect("row is attached");

        // Then the result carries a DashboardNav message.
        assert_eq!(
            result.message_names,
            vec![std::any::type_name::<super::DashboardNav>()],
            "dashboard nav message name"
        );
    }

    #[rstest::rstest]
    #[test]
    fn unbound_intent_yields_no_route() {
        // Given a route table with the dashboard rows attached.
        let routes = KeyRoutes::new();
        attach_dashboard_rows(&routes);

        // When looking up an unrelated intent.
        let result = routes.action_for(&Intent::NoOp);

        // Then no route serves it.
        assert!(result.is_none());
    }
}
