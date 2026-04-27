//! Plugin for mode-switching commands.
//!
//! Handles the `AppSetMode` command to change the interaction mode.

use nullslop_plugin::{Out, define_plugin};
use nullslop_protocol::CommandAction;
use nullslop_protocol::command::AppSetMode;

define_plugin! {
    /// Handles mode-switching commands.
    pub(crate) struct NormalModePlugin;

    commands {
        AppSetMode: on_set_mode,
    }

    events {}
}

impl NormalModePlugin {
    #[allow(clippy::unused_self, clippy::trivially_copy_pass_by_ref)]
    fn on_set_mode(
        &self,
        cmd: &AppSetMode,
        state: &mut nullslop_protocol::AppData,
        _out: &mut Out,
    ) -> CommandAction {
        state.mode = cmd.mode;
        CommandAction::Continue
    }
}

#[cfg(test)]
mod tests {
    use nullslop_plugin::Bus;
    use nullslop_protocol::Command;
    use nullslop_protocol::command::AppSetMode;

    use super::*;

    #[test]
    fn set_mode_changes_app_mode() {
        // Given a bus with NormalModePlugin registered.
        let mut bus = Bus::new();
        NormalModePlugin.register(&mut bus);

        // When processing AppSetMode(Input).
        bus.submit_command(Command::AppSetMode {
            payload: AppSetMode {
                mode: nullslop_protocol::Mode::Input,
            },
        });
        let mut state = nullslop_protocol::AppData::new();
        bus.process_commands(&mut state);

        // Then state mode is Input.
        assert_eq!(state.mode, nullslop_protocol::Mode::Input);
    }
}
