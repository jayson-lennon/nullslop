//! Reacts to extension lifecycle events during shutdown.
//!
//! Keeps track of which extensions are running, notices when shutdown is requested,
//! waits for each extension to finish, and signals the application to proceed once
//! all extensions have completed.

use crate::AppState;
use npr::command::ProceedWithShutdown;
use npr::event::{
    EventApplicationShuttingDown, ExtensionShutdownCompleted, ExtensionStarted, ExtensionStarting,
};
use nullslop_component_core::{Out, define_handler};
use nullslop_protocol as npr;

define_handler! {
    pub(crate) struct ShutdownTrackerHandler;

    commands {}

    events {
        ExtensionStarting: on_extension_starting,
        ExtensionStarted: on_extension_started,
        ExtensionShutdownCompleted: on_extension_shutdown_completed,
        EventApplicationShuttingDown: on_application_shutting_down,
    }
}

impl ShutdownTrackerHandler {
    fn on_extension_starting(evt: &ExtensionStarting, state: &mut AppState, _out: &mut Out) {
        state.shutdown_tracker.track(&evt.name);
        tracing::info!(name = %evt.name, "extension starting");
    }

    fn on_extension_started(evt: &ExtensionStarted, _state: &mut AppState, _out: &mut Out) {
        tracing::info!(name = %evt.name, "extension started");
    }

    fn on_extension_shutdown_completed(
        evt: &ExtensionShutdownCompleted,
        state: &mut AppState,
        out: &mut Out,
    ) {
        let was_tracked = state.shutdown_tracker.complete(&evt.name);
        if was_tracked {
            tracing::info!(name = %evt.name, "extension shutdown completed");
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

    fn on_application_shutting_down(
        _evt: &EventApplicationShuttingDown,
        state: &mut AppState,
        _out: &mut Out,
    ) {
        state.shutdown_tracker.shutdown_active = true;
        tracing::info!("application shutting down");
    }
}

#[cfg(test)]
mod tests {
    use crate::AppState;
    use npr::Command;
    use npr::event::{ExtensionShutdownCompleted, ExtensionStarting};
    use nullslop_component_core::Bus;
    use nullslop_protocol::Event;

    use super::*;

    #[test]
    fn shutdown_tracker_tracks_starting_extension() {
        // Given a bus with ShutdownTrackerHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
        ShutdownTrackerHandler.register(&mut bus);

        // When an ExtensionStarting event is processed.
        bus.submit_event(Event::EventExtensionStarting {
            payload: ExtensionStarting {
                name: "ext-a".into(),
            },
        });
        let mut state = AppState::new();
        bus.process_events(&mut state);

        // Then the extension is in the tracker's pending set.
        assert_eq!(
            state.shutdown_tracker.pending_names(),
            vec!["ext-a".to_string()]
        );
    }

    #[test]
    fn shutdown_tracker_completes_on_last_shutdown() {
        // Given a bus with ShutdownTrackerHandler registered and one tracked extension.
        let mut bus: Bus<AppState> = Bus::new();
        ShutdownTrackerHandler.register(&mut bus);
        let mut state = AppState::new();
        state.shutdown_tracker.track("ext-a");
        state.shutdown_tracker.shutdown_active = true;

        // When the extension completes shutdown.
        bus.submit_event(Event::EventExtensionShutdownCompleted {
            payload: ExtensionShutdownCompleted {
                name: "ext-a".into(),
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
        // Given a bus with ShutdownTrackerHandler, one tracked extension, and shutdown active.
        let mut bus: Bus<AppState> = Bus::new();
        ShutdownTrackerHandler.register(&mut bus);
        let mut state = AppState::new();
        state.shutdown_tracker.track("ext-a");
        state.shutdown_tracker.shutdown_active = true;

        // When an untracked extension completes shutdown.
        bus.submit_event(Event::EventExtensionShutdownCompleted {
            payload: ExtensionShutdownCompleted {
                name: "unknown".into(),
            },
        });
        bus.process_events(&mut state);

        // Then no ProceedWithShutdown command was submitted (ext-a is still pending).
        assert!(!bus.has_pending());
        assert_eq!(
            state.shutdown_tracker.pending_names(),
            vec!["ext-a".to_string()]
        );
    }

    #[test]
    fn shutdown_tracker_sets_active_on_shutting_down() {
        // Given a bus with ShutdownTrackerHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
        ShutdownTrackerHandler.register(&mut bus);

        // When an EventApplicationShuttingDown event is processed.
        bus.submit_event(Event::EventApplicationShuttingDown);
        let mut state = AppState::new();
        bus.process_events(&mut state);

        // Then shutdown_active is true.
        assert!(state.shutdown_tracker.shutdown_active);
    }

    #[test]
    fn shutdown_tracker_not_complete_until_active() {
        // Given a bus with ShutdownTrackerHandler registered and one tracked extension.
        let mut bus: Bus<AppState> = Bus::new();
        ShutdownTrackerHandler.register(&mut bus);
        let mut state = AppState::new();
        state.shutdown_tracker.track("ext-a");
        // shutdown_active is false (default).

        // When the extension completes shutdown (but shutdown is not active).
        bus.submit_event(Event::EventExtensionShutdownCompleted {
            payload: ExtensionShutdownCompleted {
                name: "ext-a".into(),
            },
        });
        bus.process_events(&mut state);

        // Then no ProceedWithShutdown command was submitted.
        let commands = bus.drain_processed_commands();
        assert!(commands.is_empty());
    }
}
