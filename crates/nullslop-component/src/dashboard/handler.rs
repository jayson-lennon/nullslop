//! Dashboard handler — listens to extension lifecycle events.
//!
//! Tracks which extensions are starting up and which have finished starting,
//! updating the dashboard state accordingly.

use crate::AppState;
use nullslop_component_core::{Out, define_handler};
use nullslop_protocol::event::{ExtensionStarted, ExtensionStarting};

define_handler! {
    pub(crate) struct DashboardHandler;

    commands {}

    events {
        ExtensionStarting: on_extension_starting,
        ExtensionStarted: on_extension_started,
    }
}

impl DashboardHandler {
    fn on_extension_starting(evt: &ExtensionStarting, state: &mut AppState, _out: &mut Out) {
        state.dashboard.mark_starting(&evt.name);
    }

    fn on_extension_started(evt: &ExtensionStarted, state: &mut AppState, _out: &mut Out) {
        state.dashboard.mark_started(&evt.name);
    }
}

#[cfg(test)]
mod tests {
    use crate::AppState;
    use crate::dashboard::state::ExtensionStatus;
    use nullslop_component_core::Bus;
    use nullslop_protocol::Event;
    use nullslop_protocol::event::{ExtensionStarted, ExtensionStarting};

    use super::*;

    #[test]
    fn extension_starting_adds_with_starting_status() {
        // Given a bus with DashboardHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
        DashboardHandler.register(&mut bus);

        // When an ExtensionStarting event is processed.
        bus.submit_event(Event::EventExtensionStarting {
            payload: ExtensionStarting {
                name: "ext-a".into(),
            },
        });
        let mut state = AppState::new();
        bus.process_events(&mut state);

        // Then the extension is tracked with Starting status.
        let exts = state.dashboard.extensions();
        assert_eq!(exts.len(), 1);
        assert_eq!(exts[0], ("ext-a", ExtensionStatus::Starting));
    }

    #[test]
    fn extension_started_updates_to_started() {
        // Given a bus with DashboardHandler registered and an extension that has started.
        let mut bus: Bus<AppState> = Bus::new();
        DashboardHandler.register(&mut bus);
        let mut state = AppState::new();
        state.dashboard.mark_starting("ext-a");

        // When an ExtensionStarted event is processed.
        bus.submit_event(Event::EventExtensionStarted {
            payload: ExtensionStarted {
                name: "ext-a".into(),
            },
        });
        bus.process_events(&mut state);

        // Then the extension is updated to Started status.
        let exts = state.dashboard.extensions();
        assert_eq!(exts[0], ("ext-a", ExtensionStatus::Started));
    }

    #[test]
    fn multiple_extensions_tracked_in_order() {
        // Given a bus with DashboardHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
        DashboardHandler.register(&mut bus);

        // When two extensions start in sequence.
        bus.submit_event(Event::EventExtensionStarting {
            payload: ExtensionStarting {
                name: "alpha".into(),
            },
        });
        bus.submit_event(Event::EventExtensionStarted {
            payload: ExtensionStarted {
                name: "alpha".into(),
            },
        });
        bus.submit_event(Event::EventExtensionStarting {
            payload: ExtensionStarting {
                name: "beta".into(),
            },
        });
        let mut state = AppState::new();
        bus.process_events(&mut state);

        // Then both are tracked in order with correct statuses.
        let exts = state.dashboard.extensions();
        assert_eq!(exts.len(), 2);
        assert_eq!(exts[0], ("alpha", ExtensionStatus::Started));
        assert_eq!(exts[1], ("beta", ExtensionStatus::Starting));
    }
}
