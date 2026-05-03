//! Handles the quit request.
//!
//! When the user asks to quit, the application is flagged for exit and command
//! processing stops immediately, preventing any remaining handlers from running.

use crate::AppState;
use npr::CommandAction;
use npr::system::Quit;
use nullslop_component_core::{HandlerContext, define_handler};
use nullslop_protocol as npr;
use nullslop_services::Services;

define_handler! {
    pub(crate) struct AppQuitHandler;

    commands {
        Quit: on_quit,
    }

    events {}
}

impl AppQuitHandler {
    /// Handles the Quit command — flags the application for exit and stops processing.
    fn on_quit(_cmd: &Quit, ctx: &mut HandlerContext<'_, AppState, Services>) -> CommandAction {
        ctx.state.should_quit = true;
        CommandAction::Stop
    }
}

#[cfg(test)]
mod tests {
    use crate::AppState;
    use npr::Command;
    use nullslop_component_core::Bus;
    use nullslop_protocol as npr;
    use nullslop_services::Services;

    use super::*;
    use crate::test_utils;

    #[test]
    fn quit_sets_should_quit() {
        // Given a bus with AppQuitHandler registered.
        let mut bus: Bus<AppState, Services> = Bus::new();
        AppQuitHandler.register(&mut bus);

        // When processing Quit.
        bus.submit_command(Command::Quit);
        let services = test_utils::test_services();
        let mut state = AppState::default();
        bus.process_commands(&mut state, &services);

        // Then should_quit is true.
        assert!(state.should_quit);
    }
}
