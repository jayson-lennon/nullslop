//! Dashboard handler — listens to actor lifecycle events.
//!
//! Tracks which actors are starting up and which have finished starting,
//! updating the dashboard state accordingly.

use crate::AppState;
use nullslop_component_core::{Out, define_handler};
use nullslop_protocol::actor::{ActorStarted, ActorStarting};

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
    fn on_actor_starting(evt: &ActorStarting, state: &mut AppState, _out: &mut Out) {
        state.dashboard.mark_starting(&evt.name);
    }

    /// Records an actor as started in the dashboard state.
    fn on_actor_started(evt: &ActorStarted, state: &mut AppState, _out: &mut Out) {
        state.dashboard.mark_started(&evt.name);
    }
}

#[cfg(test)]
mod tests {
    use crate::AppState;
    use crate::dashboard::state::ActorStatus;
    use nullslop_component_core::Bus;
    use nullslop_protocol::Event;
    use nullslop_protocol::actor::{ActorStarted, ActorStarting};

    use super::*;

    #[test]
    fn actor_starting_adds_with_starting_status() {
        // Given a bus with DashboardHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
        DashboardHandler.register(&mut bus);

        // When an ActorStarting event is processed.
        bus.submit_event(Event::ActorStarting {
            payload: ActorStarting {
                name: "actor-a".into(),
            },
        });
        let mut state = AppState::new();
        bus.process_events(&mut state);

        // Then the actor is tracked with Starting status.
        let actors = state.dashboard.actors();
        assert_eq!(actors.len(), 1);
        assert_eq!(actors[0], ("actor-a", ActorStatus::Starting));
    }

    #[test]
    fn actor_started_updates_to_started() {
        // Given a bus with DashboardHandler registered and an actor that has started.
        let mut bus: Bus<AppState> = Bus::new();
        DashboardHandler.register(&mut bus);
        let mut state = AppState::new();
        state.dashboard.mark_starting("actor-a");

        // When an ActorStarted event is processed.
        bus.submit_event(Event::ActorStarted {
            payload: ActorStarted {
                name: "actor-a".into(),
            },
        });
        bus.process_events(&mut state);

        // Then the actor is updated to Started status.
        let actors = state.dashboard.actors();
        assert_eq!(actors[0], ("actor-a", ActorStatus::Started));
    }

    #[test]
    fn multiple_actors_tracked_in_order() {
        // Given a bus with DashboardHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
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
        let mut state = AppState::new();
        bus.process_events(&mut state);

        // Then both are tracked in order with correct statuses.
        let actors = state.dashboard.actors();
        assert_eq!(actors.len(), 2);
        assert_eq!(actors[0], ("alpha", ActorStatus::Started));
        assert_eq!(actors[1], ("beta", ActorStatus::Starting));
    }
}
