//! Plugin for core application commands.
//!
//! Handles the `AppQuit` command by setting the `should_quit` flag
//! and stopping command propagation.

use nullslop_plugin::{Out, define_plugin};
use nullslop_protocol::CommandAction;
use nullslop_protocol::command::AppQuit;

define_plugin! {
    /// Handles core application commands.
    pub(crate) struct CoreDispatcher;

    commands {
        AppQuit: on_quit,
    }

    events {}
}

impl CoreDispatcher {
    #[allow(clippy::unused_self, clippy::trivially_copy_pass_by_ref)]
    fn on_quit(
        &self,
        _cmd: &AppQuit,
        state: &mut nullslop_protocol::AppData,
        _out: &mut Out,
    ) -> CommandAction {
        state.should_quit = true;
        CommandAction::Stop
    }
}

#[cfg(test)]
mod tests {
    use nullslop_plugin::Bus;
    use nullslop_protocol::Command;

    use super::*;

    #[test]
    fn quit_sets_should_quit() {
        // Given a bus with CoreDispatcher registered.
        let mut bus = Bus::new();
        CoreDispatcher.register(&mut bus);

        // When processing AppQuit.
        bus.submit_command(Command::AppQuit);
        let mut state = nullslop_protocol::AppData::new();
        bus.process_commands(&mut state);

        // Then should_quit is true.
        assert!(state.should_quit);
    }
}
