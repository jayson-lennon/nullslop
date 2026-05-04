//! Picker handler — processes provider picker commands.
//!
//! Handles filter input, selection movement, and confirmation.
//! On confirmation, submits a `ProviderSwitch` command and closes the picker.

use crate::AppState;
use crate::provider_picker::entries::{filtered_entries, sorted_entries};
use npr::CommandAction;
use npr::provider::ProviderSwitch;
use npr::provider_picker::{
    PickerBackspace, PickerConfirm, PickerInsertChar, PickerMoveCursorLeft, PickerMoveCursorRight,
    PickerMoveDown, PickerMoveUp,
};
use npr::system::SetMode;
use nullslop_component_core::{HandlerContext, define_handler};
use nullslop_protocol as npr;
use nullslop_services::Services;

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
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        ctx.state.picker.insert_char(cmd.ch);
        CommandAction::Continue
    }

    /// Deletes the last character from the picker filter.
    fn on_backspace(
        _cmd: &PickerBackspace,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        ctx.state.picker.backspace();
        CommandAction::Continue
    }

    /// Confirms the current picker selection.
    ///
    /// Submits `ProviderSwitch` if the selected entry is available,
    /// then closes the picker by setting mode to Normal.
    fn on_confirm(
        _cmd: &PickerConfirm,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        let services = ctx.services;
        let registry = services.provider_registry().read();
        let api_keys = services.api_keys().read();
        let unsorted = filtered_entries(
            &registry,
            &api_keys,
            &ctx.state.picker.filter,
            ctx.state.model_cache.as_ref(),
        );
        let entries = sorted_entries(
            &unsorted,
            &ctx.state.picker.filter,
            &ctx.state.active_provider,
        );

        let Some(entry) = entries.get(ctx.state.picker.selection) else {
            return CommandAction::Continue;
        };

        if !entry.is_available {
            // Unavailable provider selected — do nothing.
            return CommandAction::Continue;
        }

        let provider_id = entry.provider_id.clone();

        // Submit provider switch.
        ctx.out.submit_command(npr::Command::ProviderSwitch {
            payload: ProviderSwitch { provider_id },
        });

        // Close picker.
        ctx.out.submit_command(npr::Command::SetMode {
            payload: SetMode {
                mode: npr::Mode::Normal,
            },
        });

        CommandAction::Continue
    }

    /// Moves the picker selection up.
    fn on_move_up(
        _cmd: &PickerMoveUp,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        let count = picker_entry_count(ctx.services, &*ctx.state);
        ctx.state.picker.move_up(count, PICKER_MAX_VISIBLE);
        CommandAction::Continue
    }

    /// Moves the picker selection down.
    fn on_move_down(
        _cmd: &PickerMoveDown,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        let count = picker_entry_count(ctx.services, &*ctx.state);
        ctx.state.picker.move_down(count, PICKER_MAX_VISIBLE);
        CommandAction::Continue
    }

    /// Moves the picker filter cursor left.
    fn on_move_cursor_left(
        _cmd: &PickerMoveCursorLeft,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        ctx.state.picker.move_cursor_left();
        CommandAction::Continue
    }

    /// Moves the picker filter cursor right.
    fn on_move_cursor_right(
        _cmd: &PickerMoveCursorRight,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        ctx.state.picker.move_cursor_right();
        CommandAction::Continue
    }
}

/// Maximum number of visible result rows used for scroll clamping in the handler.
/// The actual visible rows are determined dynamically by the renderer based on
/// terminal height. This value is a generous upper bound so the handler's scroll
/// offset tracking stays reasonable.
const PICKER_MAX_VISIBLE: usize = 100;
/// Counts the number of picker entries matching the current filter.
fn picker_entry_count(services: &Services, state: &AppState) -> usize {
    let registry = services.provider_registry().read();
    let api_keys = services.api_keys().read();
    filtered_entries(
        &registry,
        &api_keys,
        &state.picker.filter,
        state.model_cache.as_ref(),
    )
    .len()
}


