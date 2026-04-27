//! Plugin for core application commands.
//!
//! Handles the `AppQuit` command by setting the `should_quit` flag
//! and stopping command propagation.

use npr::CommandAction;
use npr::command::AppQuit;
use nullslop_plugin::{Out, define_plugin};
use nullslop_protocol as npr;

define_plugin! {
    /// Handles core application commands.
    pub(crate) struct CoreDispatcher;

    commands {
        AppQuit: on_quit,
    }

    events {}
}

impl CoreDispatcher {
    fn on_quit(_cmd: &AppQuit, state: &mut npr::AppData, _out: &mut Out) -> CommandAction {
        state.should_quit = true;
        CommandAction::Stop
    }
}

#[cfg(test)]
mod tests {
    use npr::Command;
    use nullslop_plugin::Bus;
    use nullslop_protocol as npr;

    use super::*;

    #[test]
    fn quit_sets_should_quit() {
        // Given a bus with CoreDispatcher registered.
        let mut bus = Bus::new();
        CoreDispatcher.register(&mut bus);

        // When processing AppQuit.
        bus.submit_command(Command::AppQuit);
        let mut state = npr::AppData::new();
        bus.process_commands(&mut state);

        // Then should_quit is true.
        assert!(state.should_quit);
    }
}
