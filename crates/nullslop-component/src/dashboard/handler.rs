//! Dashboard handler — listens to actor lifecycle events.
//!
//! Tracks which actors are starting up and which have finished starting,
//! updating the dashboard state accordingly.

use crate::AppState;
use nullslop_component_core::{HandlerContext, define_handler};
use nullslop_protocol::actor::{ActorStarted, ActorStarting};
use nullslop_services::Services;

define_handler! {
    pub(crate) struct DashboardHandler;

    commands {}

    events {
        ActorStarting: on_actor_starting,
        ActorStarted: on_actor_started,
    }
}

impl DashboardHandler {
    /// Records an actor as starting in the dashboard state.
    fn on_actor_starting(evt: &ActorStarting, ctx: &mut HandlerContext<'_, AppState, Services>) {
        ctx.state.dashboard.mark_starting(&evt.name);
    }

    /// Records an actor as running in the dashboard state.
    fn on_actor_started(evt: &ActorStarted, ctx: &mut HandlerContext<'_, AppState, Services>) {
        ctx.state.dashboard.mark_running(&evt.name);
    }
}

#[cfg(test)]
mod tests {
    use crate::AppState;
    use crate::dashboard::state::ActorStatus;
    use nullslop_component_core::Bus;
    use nullslop_protocol::Event;
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
            },
        });
        let services = test_utils::test_services();
        let mut state = AppState::default();
        bus.process_events(&mut state, &services);

        // Then the actor is tracked with Starting status.
        let actors = state.dashboard.actors();
        assert_eq!(actors.len(), 1);
        assert_eq!(actors[0], ("actor-a", ActorStatus::Starting));
    }

    #[test]
    fn actor_started_updates_to_running() {
        // Given a bus with DashboardHandler registered and an actor that is running.
        let mut bus: Bus<AppState, Services> = Bus::new();
        DashboardHandler.register(&mut bus);
        let services = test_utils::test_services();
        let mut state = AppState::default();
        state.dashboard.mark_starting("actor-a");

        // When an ActorStarted event is processed.
        bus.submit_event(Event::ActorStarted {
            payload: ActorStarted {
                name: "actor-a".into(),
            },
        });
        bus.process_events(&mut state, &services);

        // Then the actor is updated to Running status.
        let actors = state.dashboard.actors();
        assert_eq!(actors[0], ("actor-a", ActorStatus::Running));
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
            },
        });
        bus.submit_event(Event::ActorStarted {
            payload: ActorStarted {
                name: "alpha".into(),
            },
        });
        bus.submit_event(Event::ActorStarting {
            payload: ActorStarting {
                name: "beta".into(),
            },
        });
        let services = test_utils::test_services();
        let mut state = AppState::default();
        bus.process_events(&mut state, &services);

        // Then both are tracked in order with correct statuses.
        let actors = state.dashboard.actors();
        assert_eq!(actors.len(), 2);
        assert_eq!(actors[0], ("alpha", ActorStatus::Running));
        assert_eq!(actors[1], ("beta", ActorStatus::Starting));
    }
}
