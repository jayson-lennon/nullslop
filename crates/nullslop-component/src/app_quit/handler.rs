//! Handles the quit request.
//!
//! When the user asks to quit, the application is flagged for exit and command
//! processing stops immediately, preventing any remaining handlers from running.

use crate::AppState;
use npr::CommandAction;
use npr::command::AppQuit;
use nullslop_component_core::{Out, define_handler};
use nullslop_protocol as npr;

define_handler! {
    pub(crate) struct AppQuitHandler;

    commands {
        AppQuit: on_quit,
    }

    events {}
}

impl AppQuitHandler {
    fn on_quit(_cmd: &AppQuit, state: &mut AppState, _out: &mut Out) -> CommandAction {
        state.should_quit = true;
        CommandAction::Stop
    }
}

#[cfg(test)]
mod tests {
    use crate::AppState;
    use npr::Command;
    use nullslop_component_core::Bus;
    use nullslop_protocol as npr;

    use super::*;

    #[test]
    fn quit_sets_should_quit() {
        // Given a bus with AppQuitHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
        AppQuitHandler.register(&mut bus);

        // When processing AppQuit.
        bus.submit_command(Command::AppQuit);
        let mut state = AppState::new();
        bus.process_commands(&mut state);

        // Then should_quit is true.
        assert!(state.should_quit);
    }
}
