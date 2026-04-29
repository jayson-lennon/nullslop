//! Component for coordinating extension shutdown.
//!
//! Subscribes to extension lifecycle events and emits `ProceedWithShutdown`
//! when all tracked extensions have completed shutdown.

use npr::AppState;
use npr::command::ProceedWithShutdown;
use npr::event::{
    EventApplicationShuttingDown, ExtensionShutdownCompleted, ExtensionStarted, ExtensionStarting,
};
use nullslop_component_core::{Bus, Out, define_handler};
use nullslop_component_ui::UiRegistry;
use nullslop_protocol as npr;

define_handler! {
    /// Coordinates extension shutdown lifecycle.
    pub(crate) struct ShutdownComponent;

    commands {}

    events {
        ExtensionStarting: on_extension_starting,
        ExtensionStarted: on_extension_started,
        ExtensionShutdownCompleted: on_extension_shutdown_completed,
        EventApplicationShuttingDown: on_application_shutting_down,
    }
}

/// Register the shutdown component.
pub(crate) fn register(bus: &mut Bus, _registry: &mut UiRegistry) {
    ShutdownComponent.register(bus);
}

impl ShutdownComponent {
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
    use npr::Command;
    use npr::event::{ExtensionShutdownCompleted, ExtensionStarting};
    use nullslop_component_core::Bus;
    use nullslop_protocol::{AppState, Event};

    use super::*;

    #[test]
    fn shutdown_component_tracks_starting_extension() {
        // Given a bus with ShutdownComponent registered.
        let mut bus = Bus::new();
        ShutdownComponent.register(&mut bus);

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
    fn shutdown_component_completes_on_last_shutdown() {
        // Given a bus with ShutdownComponent registered and one tracked extension.
        let mut bus = Bus::new();
        ShutdownComponent.register(&mut bus);
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
            commands[0],
            (Command::ProceedWithShutdown { .. }, _)
        ));
    }

    #[test]
    fn shutdown_component_ignores_unknown_completion() {
        // Given a bus with ShutdownComponent, one tracked extension, and shutdown active.
        let mut bus = Bus::new();
        ShutdownComponent.register(&mut bus);
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
    fn shutdown_component_sets_active_on_shutting_down() {
        // Given a bus with ShutdownComponent registered.
        let mut bus = Bus::new();
        ShutdownComponent.register(&mut bus);

        // When an EventApplicationShuttingDown event is processed.
        bus.submit_event(Event::EventApplicationShuttingDown);
        let mut state = AppState::new();
        bus.process_events(&mut state);

        // Then shutdown_active is true.
        assert!(state.shutdown_tracker.shutdown_active);
    }

    #[test]
    fn shutdown_component_not_complete_until_active() {
        // Given a bus with ShutdownComponent registered and one tracked extension.
        let mut bus = Bus::new();
        ShutdownComponent.register(&mut bus);
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
