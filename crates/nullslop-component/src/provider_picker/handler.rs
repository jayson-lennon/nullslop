//! Picker handler — processes provider picker commands.
//!
//! Handles filter input, selection movement, and confirmation.
//! On confirmation, submits a `ProviderSwitch` command and closes the picker.

use crate::AppState;
use crate::provider_picker::entries::{filtered_entries, sorted_entries};
use npr::CommandAction;
use npr::provider::ProviderSwitch;
use npr::provider_picker::{
    PickerBackspace, PickerConfirm, PickerInsertChar, PickerMoveCursorLeft,
    PickerMoveCursorRight, PickerMoveDown, PickerMoveUp,
};
use npr::system::SetMode;
use nullslop_component_core::{Out, define_handler};
use nullslop_protocol as npr;

define_handler! {
    pub(crate) struct PickerHandler;

    commands {
        PickerInsertChar: on_insert_char,
        PickerBackspace: on_backspace,
        PickerConfirm: on_confirm,
        PickerMoveUp: on_move_up,
        PickerMoveDown: on_move_down,
        PickerMoveCursorLeft: on_move_cursor_left,
        PickerMoveCursorRight: on_move_cursor_right,
    }

    events {}
}

impl PickerHandler {
    /// Inserts a character into the picker filter.
    fn on_insert_char(
        cmd: &PickerInsertChar,
        state: &mut AppState,
        _out: &mut Out,
    ) -> CommandAction {
        state.picker.insert_char(cmd.ch);
        CommandAction::Continue
    }

    /// Deletes the last character from the picker filter.
    fn on_backspace(_cmd: &PickerBackspace, state: &mut AppState, _out: &mut Out) -> CommandAction {
        state.picker.backspace();
        CommandAction::Continue
    }

    /// Confirms the current picker selection.
    ///
    /// Submits `ProviderSwitch` if the selected entry is available,
    /// then closes the picker by setting mode to Normal.
    fn on_confirm(_cmd: &PickerConfirm, state: &mut AppState, out: &mut Out) -> CommandAction {
        let services = &state.services;
        let registry = services.provider_registry().read();
        let api_keys = services.api_keys().read();
        let entries = sorted_entries(
            filtered_entries(&registry, &api_keys, &state.picker.filter),
            &state.picker.filter,
            &state.active_provider,
        );

        let Some(entry) = entries.get(state.picker.selection) else {
            return CommandAction::Continue;
        };

        if !entry.is_available {
            // Unavailable provider selected — do nothing.
            return CommandAction::Continue;
        }

        let provider_id = entry.provider_id.clone();

        // Submit provider switch.
        out.submit_command(npr::Command::ProviderSwitch {
            payload: ProviderSwitch { provider_id },
        });

        // Close picker.
        out.submit_command(npr::Command::SetMode {
            payload: SetMode {
                mode: npr::Mode::Normal,
            },
        });

        CommandAction::Continue
    }

    /// Moves the picker selection up.
    fn on_move_up(_cmd: &PickerMoveUp, state: &mut AppState, _out: &mut Out) -> CommandAction {
        let count = picker_entry_count(state);
        state.picker.move_up(count, PICKER_MAX_VISIBLE);
        CommandAction::Continue
    }

    /// Moves the picker selection down.
    fn on_move_down(_cmd: &PickerMoveDown, state: &mut AppState, _out: &mut Out) -> CommandAction {
        let count = picker_entry_count(state);
        state.picker.move_down(count, PICKER_MAX_VISIBLE);
        CommandAction::Continue
    }

    /// Moves the picker filter cursor left.
    fn on_move_cursor_left(
        _cmd: &PickerMoveCursorLeft,
        state: &mut AppState,
        _out: &mut Out,
    ) -> CommandAction {
        state.picker.move_cursor_left();
        CommandAction::Continue
    }

    /// Moves the picker filter cursor right.
    fn on_move_cursor_right(
        _cmd: &PickerMoveCursorRight,
        state: &mut AppState,
        _out: &mut Out,
    ) -> CommandAction {
        state.picker.move_cursor_right();
        CommandAction::Continue
    }
}

/// Maximum number of visible result rows in the picker popup.
/// Must match the value used by the renderer.
const PICKER_MAX_VISIBLE: usize = 8;
fn picker_entry_count(state: &AppState) -> usize {
    let services = &state.services;
    let registry = services.provider_registry().read();
    let api_keys = services.api_keys().read();
    filtered_entries(&registry, &api_keys, &state.picker.filter).len()
}

#[cfg(test)]
mod tests {
    use crate::AppState;
    use npr::Command;
    use npr::provider_picker::PickerInsertChar;
    use nullslop_component_core::Bus;
    use nullslop_protocol as npr;
    use nullslop_providers::{
        ApiKeys, ApiKeysService, ProviderEntry, ProviderRegistry, ProviderRegistryService,
        ProvidersConfig,
    };

    use super::*;
    use crate::test_utils;
    fn state_with_ollama() -> AppState {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        let guard = rt.enter();

        let config = ProvidersConfig {
            providers: vec![ProviderEntry {
                name: "ollama".to_owned(),
                backend: "ollama".to_owned(),
                models: vec!["llama3".to_owned()],
                base_url: Some("http://localhost:11434".to_owned()),
                api_key_env: None,
                requires_key: false,
            }],
            aliases: vec![],
            default_provider: None,
        };
        let registry =
            ProviderRegistryService::new(ProviderRegistry::from_config(config).expect("registry"));
        let api_keys = ApiKeysService::new(ApiKeys::new());
        let services = nullslop_services::Services::new(
            tokio::runtime::Handle::current(),
            std::sync::Arc::new(
                nullslop_actor_host::InMemoryActorHost::from_actors_with_handle(
                    vec![],
                    tokio::runtime::Handle::current(),
                ),
            ),
            nullslop_providers::LlmServiceFactoryService::new(std::sync::Arc::new(
                nullslop_providers::FakeLlmServiceFactory::new(vec![]),
            )),
            registry,
            api_keys,
            nullslop_providers::ConfigStorageService::new(std::sync::Arc::new(
                nullslop_providers::InMemoryConfigStorage::new(),
            )),
        );
        let state = AppState::new(services);
        drop(guard);
        drop(rt);
        state
    }

    /// Creates an `AppState` with a key-required provider but no key set.
    fn state_with_unavailable() -> AppState {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        let guard = rt.enter();

        let config = ProvidersConfig {
            providers: vec![ProviderEntry {
                name: "openrouter".to_owned(),
                backend: "openrouter".to_owned(),
                models: vec!["gpt-4".to_owned()],
                base_url: None,
                api_key_env: Some("OPENROUTER_API_KEY".to_owned()),
                requires_key: true,
            }],
            aliases: vec![],
            default_provider: None,
        };
        let registry =
            ProviderRegistryService::new(ProviderRegistry::from_config(config).expect("registry"));
        let api_keys = ApiKeysService::new(ApiKeys::new());
        let services = nullslop_services::Services::new(
            tokio::runtime::Handle::current(),
            std::sync::Arc::new(
                nullslop_actor_host::InMemoryActorHost::from_actors_with_handle(
                    vec![],
                    tokio::runtime::Handle::current(),
                ),
            ),
            nullslop_providers::LlmServiceFactoryService::new(std::sync::Arc::new(
                nullslop_providers::FakeLlmServiceFactory::new(vec![]),
            )),
            registry,
            api_keys,
            nullslop_providers::ConfigStorageService::new(std::sync::Arc::new(
                nullslop_providers::InMemoryConfigStorage::new(),
            )),
        );
        let state = AppState::new(services);
        drop(guard);
        drop(rt);
        state
    }

    #[test]
    fn insert_char_updates_picker_filter() {
        // Given a bus with PickerHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
        PickerHandler.register(&mut bus);

        // When processing PickerInsertChar('o').
        bus.submit_command(Command::PickerInsertChar {
            payload: PickerInsertChar { ch: 'o' },
        });
        let mut state = AppState::new(test_utils::test_services());
        bus.process_commands(&mut state);

        // Then the picker filter is "o".
        assert_eq!(state.picker.filter, "o");
    }

    #[test]
    fn backspace_removes_from_filter() {
        // Given a bus with PickerHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
        PickerHandler.register(&mut bus);

        // When processing PickerInsertChar('o') then PickerInsertChar('l') then PickerBackspace.
        bus.submit_command(Command::PickerInsertChar {
            payload: PickerInsertChar { ch: 'o' },
        });
        bus.submit_command(Command::PickerInsertChar {
            payload: PickerInsertChar { ch: 'l' },
        });
        bus.submit_command(Command::PickerBackspace);
        let mut state = AppState::new(test_utils::test_services());
        bus.process_commands(&mut state);

        // Then the filter is "o".
        assert_eq!(state.picker.filter, "o");
    }

    #[test]
    fn move_up_decrements_selection() {
        // Given a bus with PickerHandler registered and selection at 2.
        let mut bus: Bus<AppState> = Bus::new();
        PickerHandler.register(&mut bus);

        let mut state = state_with_ollama();
        state.picker.selection = 1;

        // When processing PickerMoveUp.
        bus.submit_command(Command::PickerMoveUp);
        bus.process_commands(&mut state);

        // Then selection is 0.
        assert_eq!(state.picker.selection, 0);
    }

    #[test]
    fn move_down_increments_selection() {
        // Given a bus with PickerHandler registered and selection at 0.
        let mut bus: Bus<AppState> = Bus::new();
        PickerHandler.register(&mut bus);

        let mut state = state_with_ollama();

        // When processing PickerMoveDown.
        bus.submit_command(Command::PickerMoveDown);
        bus.process_commands(&mut state);

        // Then selection is still clamped (only 1 entry, already at 0).
        assert_eq!(state.picker.selection, 0);
    }

    #[test]
    fn confirm_submits_provider_switch_and_closes() {
        // Given a bus with PickerHandler, SwitchHandler, and ChatInputBoxHandler registered, with "ollama" available.
        let mut bus: Bus<AppState> = Bus::new();
        PickerHandler.register(&mut bus);
        crate::provider::switch_handler::SwitchHandler.register(&mut bus);
        crate::chat_input_box::ChatInputBoxHandler.register(&mut bus);

        let mut state = state_with_ollama();
        state.mode = npr::Mode::Picker;

        // When processing PickerConfirm (ollama is selected at index 0).
        bus.submit_command(Command::PickerConfirm);
        bus.process_commands(&mut state);

        // Then a ProviderSwitch was submitted and processed.
        assert_eq!(state.active_provider, "ollama/llama3");

        // And mode is back to Normal.
        assert_eq!(state.mode, npr::Mode::Normal);
    }

    #[test]
    fn confirm_ignores_unavailable_provider() {
        // Given a bus with PickerHandler registered, with unavailable provider.
        let mut bus: Bus<AppState> = Bus::new();
        PickerHandler.register(&mut bus);
        crate::provider::switch_handler::SwitchHandler.register(&mut bus);

        let mut state = state_with_unavailable();
        state.mode = npr::Mode::Picker;

        // When processing PickerConfirm (openrouter is unavailable).
        bus.submit_command(Command::PickerConfirm);
        bus.process_commands(&mut state);

        // Then no ProviderSwitch was submitted (active_provider still NO_PROVIDER_ID).
        assert_eq!(state.active_provider, nullslop_providers::NO_PROVIDER_ID);

        // And mode is still Picker.
        assert_eq!(state.mode, npr::Mode::Picker);
    }

    #[test]
    fn move_cursor_left_decrements_cursor() {
        // Given a bus with PickerHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
        PickerHandler.register(&mut bus);

        // When inserting "ab" then moving cursor left.
        bus.submit_command(Command::PickerInsertChar {
            payload: PickerInsertChar { ch: 'a' },
        });
        bus.submit_command(Command::PickerInsertChar {
            payload: PickerInsertChar { ch: 'b' },
        });
        bus.submit_command(Command::PickerMoveCursorLeft);
        let mut state = AppState::new(test_utils::test_services());
        bus.process_commands(&mut state);

        // Then the cursor position is 1 (was at end after "ab", moved left once).
        assert_eq!(state.picker.cursor_pos(), 1);
    }

    #[test]
    fn move_cursor_right_increments_cursor() {
        // Given a bus with PickerHandler registered.
        let mut bus: Bus<AppState> = Bus::new();
        PickerHandler.register(&mut bus);

        // When inserting "ab" then moving cursor left twice (to 0) then right once.
        bus.submit_command(Command::PickerInsertChar {
            payload: PickerInsertChar { ch: 'a' },
        });
        bus.submit_command(Command::PickerInsertChar {
            payload: PickerInsertChar { ch: 'b' },
        });
        bus.submit_command(Command::PickerMoveCursorLeft);
        bus.submit_command(Command::PickerMoveCursorLeft);
        bus.submit_command(Command::PickerMoveCursorRight);
        let mut state = AppState::new(test_utils::test_services());
        bus.process_commands(&mut state);

        // Then the cursor position is 1 (was at 0, moved right once).
        assert_eq!(state.picker.cursor_pos(), 1);
    }
}
