//! Dashboard handler — listens to actor lifecycle events.
//!
//! Tracks which actors are starting up and which have finished starting,
//! updating the dashboard state accordingly.

use crate::AppState;
use nullslop_component_core::{HandlerContext, define_handler};
use nullslop_protocol::actor::{ActorStarted, ActorStarting};
use nullslop_protocol::system::{DashboardSelectDown, DashboardSelectUp};
use nullslop_protocol::{ActiveTab, CommandAction};
use nullslop_services::Services;

define_handler! {
    pub(crate) struct DashboardHandler;

    commands {
        DashboardSelectDown: on_select_down,
        DashboardSelectUp: on_select_up,
    }

    events {
        ActorStarting: on_actor_starting,
        ActorStarted: on_actor_started,
    }
}

impl DashboardHandler {
    /// Records an actor as starting in the dashboard state.
    fn on_actor_starting(evt: &ActorStarting, ctx: &mut HandlerContext<'_, AppState, Services>) {
        ctx.state.dashboard.mark_starting(&evt.name, evt.description.clone());
    }

    /// Records an actor as running in the dashboard state.
    fn on_actor_started(evt: &ActorStarted, ctx: &mut HandlerContext<'_, AppState, Services>) {
        ctx.state.dashboard.mark_running(&evt.name, evt.description.clone());
    }

    /// Moves the dashboard selection down one entry.
    ///
    /// No-op when the Dashboard tab is not active.
    fn on_select_down(_cmd: &DashboardSelectDown, ctx: &mut HandlerContext<'_, AppState, Services>) -> CommandAction {
        if ctx.state.active_tab == ActiveTab::Dashboard {
            ctx.state.dashboard.select_next();
        }
        CommandAction::Continue
    }

    /// Moves the dashboard selection up one entry.
    ///
    /// No-op when the Dashboard tab is not active.
    fn on_select_up(_cmd: &DashboardSelectUp, ctx: &mut HandlerContext<'_, AppState, Services>) -> CommandAction {
        if ctx.state.active_tab == ActiveTab::Dashboard {
            ctx.state.dashboard.select_prev();
        }
        CommandAction::Continue
    }
}

#[cfg(test)]
mod tests {
    use crate::AppState;
    use crate::dashboard::state::ActorStatus;
    use nullslop_component_core::Bus;
    use nullslop_protocol::Event;
    use nullslop_protocol::Command;
    use nullslop_protocol::actor::{ActorStarted, ActorStarting};
    use nullslop_services::Services;

    use super::*;
    use crate::test_utils;

    #[test]
    fn actor_starting_adds_with_starting_status() {
        // Given a bus with DashboardHandler registered.
        let mut bus: Bus<AppState, Services> = Bus::new();
        DashboardHandler.register(&mut bus);

        // When an ActorStarting event is processed.
        bus.submit_event(Event::ActorStarting {
            payload: ActorStarting {
                name: "actor-a".into(),
                description: None,
            },
        });
        let services = test_utils::test_services();
        let mut state = AppState::default();
        bus.process_events(&mut state, &services);

        // Then the actor is tracked with Starting status.
        let actors = state.dashboard.actors();
        assert_eq!(actors.len(), 1);
        assert_eq!(actors[0].name, "actor-a");
        assert_eq!(actors[0].status, ActorStatus::Starting);
    }

    #[test]
    fn actor_started_updates_to_running() {
        // Given a bus with DashboardHandler registered and an actor that is running.
        let mut bus: Bus<AppState, Services> = Bus::new();
        DashboardHandler.register(&mut bus);
        let services = test_utils::test_services();
        let mut state = AppState::default();
        state.dashboard.mark_starting("actor-a", None);

        // When an ActorStarted event is processed.
        bus.submit_event(Event::ActorStarted {
            payload: ActorStarted {
                name: "actor-a".into(),
                description: None,
            },
        });
        bus.process_events(&mut state, &services);

        // Then the actor is updated to Running status.
        let actors = state.dashboard.actors();
        assert_eq!(actors[0].name, "actor-a");
        assert_eq!(actors[0].status, ActorStatus::Running);
    }

    #[test]
    fn multiple_actors_tracked_in_order() {
        // Given a bus with DashboardHandler registered.
        let mut bus: Bus<AppState, Services> = Bus::new();
        DashboardHandler.register(&mut bus);

        // When two actors start in sequence.
        bus.submit_event(Event::ActorStarting {
            payload: ActorStarting {
                name: "alpha".into(),
                description: None,
            },
        });
        bus.submit_event(Event::ActorStarted {
            payload: ActorStarted {
                name: "alpha".into(),
                description: None,
            },
        });
        bus.submit_event(Event::ActorStarting {
            payload: ActorStarting {
                name: "beta".into(),
                description: None,
            },
        });
        let services = test_utils::test_services();
        let mut state = AppState::default();
        bus.process_events(&mut state, &services);

        // Then both are tracked in order with correct statuses.
        let actors = state.dashboard.actors();
        assert_eq!(actors.len(), 2);
        assert_eq!(actors[0].name, "alpha");
        assert_eq!(actors[0].status, ActorStatus::Running);
        assert_eq!(actors[1].name, "beta");
        assert_eq!(actors[1].status, ActorStatus::Starting);
    }

    #[test]
    fn select_down_moves_selection_on_dashboard_tab() {
        // Given a bus with DashboardHandler registered, on the Dashboard tab.
        let mut bus: Bus<AppState, Services> = Bus::new();
        DashboardHandler.register(&mut bus);
        let services = test_utils::test_services();
        let mut state = AppState {
            active_tab: nullslop_protocol::ActiveTab::Dashboard,
            ..Default::default()
        };
        state.dashboard.mark_starting("echo", None);
        state.dashboard.mark_starting("llm", None);

        // When processing a DashboardSelectDown command.
        bus.submit_command(Command::DashboardSelectDown);
        bus.process_commands(&mut state, &services);

        // Then the selected index is 1.
        assert_eq!(state.dashboard.selected_index(), 1);
    }

    #[test]
    fn select_up_clamps_at_zero_on_dashboard_tab() {
        // Given a bus with DashboardHandler registered, on the Dashboard tab at index 0.
        let mut bus: Bus<AppState, Services> = Bus::new();
        DashboardHandler.register(&mut bus);
        let services = test_utils::test_services();
        let mut state = AppState {
            active_tab: nullslop_protocol::ActiveTab::Dashboard,
            ..Default::default()
        };
        state.dashboard.mark_starting("echo", None);
        state.dashboard.mark_starting("llm", None);

        // When processing a DashboardSelectUp command.
        bus.submit_command(Command::DashboardSelectUp);
        bus.process_commands(&mut state, &services);

        // Then the selected index stays at 0.
        assert_eq!(state.dashboard.selected_index(), 0);
    }

    #[test]
    fn select_is_noop_on_chat_tab() {
        // Given a bus with DashboardHandler registered, on the Chat tab.
        let mut bus: Bus<AppState, Services> = Bus::new();
        DashboardHandler.register(&mut bus);
        let services = test_utils::test_services();
        let mut state = AppState::default();
        state.dashboard.mark_starting("echo", None);
        state.dashboard.mark_starting("llm", None);

        // When processing a DashboardSelectDown command.
        bus.submit_command(Command::DashboardSelectDown);
        bus.process_commands(&mut state, &services);

        // Then the selected index stays at 0.
        assert_eq!(state.dashboard.selected_index(), 0);
    }
}
