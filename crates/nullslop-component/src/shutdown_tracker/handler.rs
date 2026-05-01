//! Reacts to actor lifecycle events during shutdown.
//!
//! Keeps track of which actors are running, notices when shutdown is requested,
//! waits for each actor to finish, and signals the application to proceed once
//! all actors have completed.

use crate::AppState;
use npr::actor::ProceedWithShutdown;
use npr::actor::{ActorShutdownCompleted, ActorStarted, ActorStarting};
use nullslop_component_core::{Out, define_handler};
use nullslop_protocol as npr;

define_handler! {
    pub(crate) struct ShutdownTrackerHandler;

    commands {}

    events {
        ActorStarting: on_actor_starting,
        ActorStarted: on_actor_started,
        ActorShutdownCompleted: on_actor_shutdown_completed,
    }
}

impl ShutdownTrackerHandler {
    fn on_actor_starting(evt: &ActorStarting, state: &mut AppState, _out: &mut Out) {
        state.shutdown_tracker.track(&evt.name);
        tracing::info!(name = %evt.name, "actor starting");
    }

    fn on_actor_started(evt: &ActorStarted, _state: &mut AppState, _out: &mut Out) {
        tracing::info!(name = %evt.name, "actor started");
    }

    fn on_actor_shutdown_completed(
        evt: &ActorShutdownCompleted,
        state: &mut AppState,
        out: &mut Out,
    ) {
        let was_tracked = state.shutdown_tracker.complete(&evt.name);
        if was_tracked {
            tracing::info!(name = %evt.name, "actor shutdown completed");
        }
        if state.shutdown_tracker.is_complete() {
            out.submit_command(npr::Command::ProceedWithShutdown {
                payload: ProceedWithShutdown {
                    completed: vec![evt.name.clone()],
                    timed_out: vec![],
                },
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::AppState;
    use npr::Command;
    use npr::actor::{ActorShutdownCompleted, ActorStarting};
    use nullslop_component_core::Bus;
    use nullslop_protocol::Event;

    use super::*;

    #[test]
    fn shutdown_tracker_tracks_starting_actor() {
        // Given a bus with ShutdownTrackerHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
        ShutdownTrackerHandler.register(&mut bus);

        // When an ActorStarting event is processed.
        bus.submit_event(Event::ActorStarting {
            payload: ActorStarting {
                name: "actor-a".into(),
            },
        });
        let mut state = AppState::new();
        bus.process_events(&mut state);

        // Then the actor is in the tracker's pending set.
        assert_eq!(
            state.shutdown_tracker.pending_names(),
            vec!["actor-a".to_string()]
        );
    }

    #[test]
    fn shutdown_tracker_completes_on_last_shutdown() {
        // Given a bus with ShutdownTrackerHandler registered and one tracked actor.
        let mut bus: Bus<AppState> = Bus::new();
        ShutdownTrackerHandler.register(&mut bus);
        let mut state = AppState::new();
        state.shutdown_tracker.track("actor-a");
        state.shutdown_tracker.shutdown_active = true;

        // When the actor completes shutdown.
        bus.submit_event(Event::ActorShutdownCompleted {
            payload: ActorShutdownCompleted {
                name: "actor-a".into(),
            },
        });
        bus.process_events(&mut state);

        // Then a ProceedWithShutdown command was queued.
        assert!(bus.has_pending());
        bus.process_commands(&mut state);
        let commands = bus.drain_processed_commands();
        assert_eq!(commands.len(), 1);
        assert!(matches!(
            &commands[0].command,
            Command::ProceedWithShutdown { .. }
        ));
    }

    #[test]
    fn shutdown_tracker_ignores_unknown_completion() {
        // Given a bus with ShutdownTrackerHandler, one tracked actor, and shutdown active.
        let mut bus: Bus<AppState> = Bus::new();
        ShutdownTrackerHandler.register(&mut bus);
        let mut state = AppState::new();
        state.shutdown_tracker.track("actor-a");
        state.shutdown_tracker.shutdown_active = true;

        // When an untracked actor completes shutdown.
        bus.submit_event(Event::ActorShutdownCompleted {
            payload: ActorShutdownCompleted {
                name: "unknown".into(),
            },
        });
        bus.process_events(&mut state);

        // Then no ProceedWithShutdown command was submitted (actor-a is still pending).
        assert!(!bus.has_pending());
        assert_eq!(
            state.shutdown_tracker.pending_names(),
            vec!["actor-a".to_string()]
        );
    }

    #[test]
    fn shutdown_tracker_not_complete_until_active() {
        // Given a bus with ShutdownTrackerHandler registered and one tracked actor.
        let mut bus: Bus<AppState> = Bus::new();
        ShutdownTrackerHandler.register(&mut bus);
        let mut state = AppState::new();
        state.shutdown_tracker.track("actor-a");
        // shutdown_active is false (default).

        // When the actor completes shutdown (but shutdown is not active).
        bus.submit_event(Event::ActorShutdownCompleted {
            payload: ActorShutdownCompleted {
                name: "actor-a".into(),
            },
        });
        bus.process_events(&mut state);

        // Then no ProceedWithShutdown command was submitted.
        let commands = bus.drain_processed_commands();
        assert!(commands.is_empty());
    }
}
