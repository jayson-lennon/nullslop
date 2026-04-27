//! Plugin for mode-switching commands.
//!
//! Handles the `AppSetMode` command to change the interaction mode.

use npr::CommandAction;
use npr::command::AppSetMode;
use nullslop_plugin::{Out, define_plugin};
use nullslop_protocol as npr;

define_plugin! {
    /// Handles mode-switching commands.
    pub(crate) struct NormalModePlugin;

    commands {
        AppSetMode: on_set_mode,
    }

    events {}
}

impl NormalModePlugin {
    fn on_set_mode(cmd: &AppSetMode, state: &mut npr::AppData, _out: &mut Out) -> CommandAction {
        state.mode = cmd.mode;
        CommandAction::Continue
    }
}

#[cfg(test)]
mod tests {
    use npr::Command;
    use npr::command::AppSetMode;
    use nullslop_plugin::Bus;
    use nullslop_protocol as npr;

    use super::*;

    #[test]
    fn set_mode_changes_app_mode() {
        // Given a bus with NormalModePlugin registered.
        let mut bus = Bus::new();
        NormalModePlugin.register(&mut bus);

        // When processing AppSetMode(Input).
        bus.submit_command(Command::AppSetMode {
            payload: AppSetMode {
                mode: npr::Mode::Input,
            },
        });
        let mut state = npr::AppData::new();
        bus.process_commands(&mut state);

        // Then state mode is Input.
        assert_eq!(state.mode, npr::Mode::Input);
    }
}
