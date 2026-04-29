//! Component for the quit command.
//!
//! Handles the `AppQuit` command by setting the `should_quit` flag
//! and stopping command propagation.

use crate::AppUiRegistry;
use crate::{AppBus, AppState};
use npr::CommandAction;
use npr::command::AppQuit;
use nullslop_component_core::{Out, define_handler};
use nullslop_protocol::{self as npr};

define_handler! {
    /// Handles the quit command.
    pub(crate) struct QuitHandler;

    commands {
        AppQuit: on_quit,
    }

    events {}
}

/// Register the quit handler component.
pub(crate) fn register(bus: &mut AppBus, _: &mut AppUiRegistry) {
    QuitHandler.register(bus);
}

impl QuitHandler {
    fn on_quit(_cmd: &AppQuit, state: &mut AppState, _out: &mut Out) -> CommandAction {
        state.should_quit = true;
        CommandAction::Stop
    }
}

#[cfg(test)]
mod tests {
    use crate::AppBus;
    use npr::Command;
    use nullslop_protocol as npr;

    use super::*;

    #[test]
    fn quit_sets_should_quit() {
        // Given a bus with QuitHandler registered.
        let mut bus: AppBus = AppBus::new();
        QuitHandler.register(&mut bus);

        // When processing AppQuit.
        bus.submit_command(Command::AppQuit);
        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then should_quit is true.
        assert!(state.should_quit);
    }
}
