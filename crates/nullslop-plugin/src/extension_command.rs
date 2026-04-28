//! Plugin for commands received from extensions.
//!
//! Handles `CustomCommand` (e.g., the "echo" command) by adding
//! system chat entries.

use npr::CommandAction;
use npr::command::CustomCommand;
use nullslop_plugin_core::{Bus, Out, define_plugin};
use nullslop_plugin_ui::UiRegistry;
use nullslop_protocol as npr;

define_plugin! {
    /// Handles commands from extensions.
    pub(crate) struct ExtensionCommandPlugin;

    commands {
        CustomCommand: on_custom_command,
    }

    events {}
}

/// Register the extension command plugin.
pub(crate) fn register(bus: &mut Bus, _registry: &mut UiRegistry) {
    ExtensionCommandPlugin.register(bus);
}

impl ExtensionCommandPlugin {
    fn on_custom_command(
        cmd: &CustomCommand,
        state: &mut npr::AppData,
        _out: &mut Out,
    ) -> CommandAction {
        if cmd.name == "echo"
            && let Some(text) = cmd.args.get("text").and_then(|v| v.as_str())
        {
            state.push_entry(npr::ChatEntry::system(text));
        } else {
            tracing::warn!(name = %cmd.name, "unhandled extension command");
        }
        CommandAction::Continue
    }
}

#[cfg(test)]
mod tests {
    use npr::Command;
    use nullslop_plugin_core::Bus;
    use nullslop_protocol as npr;

    use super::*;

    #[test]
    fn echo_command_adds_system_entry() {
        // Given a bus with ExtensionCommandPlugin registered.
        let mut bus = Bus::new();
        ExtensionCommandPlugin.register(&mut bus);

        // When processing a CustomCommand "echo" with text "hello".
        bus.submit_command(Command::CustomCommand {
            payload: CustomCommand {
                name: "echo".to_string(),
                args: serde_json::json!({"text": "hello"}),
            },
        });
        let mut state = npr::AppData::new();
        bus.process_commands(&mut state);

        // Then chat_history has a System entry.
        assert_eq!(state.chat_history.len(), 1);
        assert_eq!(
            state.chat_history[0].kind,
            npr::ChatEntryKind::System("hello".to_string())
        );
    }

    #[test]
    fn unknown_command_logs_warning() {
        // Given a bus with ExtensionCommandPlugin registered.
        let mut bus = Bus::new();
        ExtensionCommandPlugin.register(&mut bus);

        // When processing an unknown CustomCommand.
        bus.submit_command(Command::CustomCommand {
            payload: CustomCommand {
                name: "unknown".to_string(),
                args: serde_json::json!({}),
            },
        });
        let mut state = npr::AppData::new();
        bus.process_commands(&mut state);

        // Then no entry is added.
        assert!(state.chat_history.is_empty());
    }
}
