//! Picker handler — processes picker commands dispatched by [`PickerKind`].
//!
//! All 7 `Picker*` commands are shared across every picker type. Each handler
//! method dispatches on [`AppState::active_picker_kind`] to route to the
//! correct [`SelectionState`] field.

use crate::AppState;
use crate::provider_picker::entries::{load_provider_entries, sorted_entries};
use npr::CommandAction;
use npr::PickerKind;
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
    /// Inserts a character into the active picker's filter.
    fn on_insert_char(
        cmd: &PickerInsertChar,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        match ctx.state.active_picker_kind {
            Some(PickerKind::Provider) => ctx.state.provider_picker.insert_char(cmd.ch),
            None => {}
        }
        CommandAction::Continue
    }

    /// Deletes the last character from the active picker's filter.
    fn on_backspace(
        _cmd: &PickerBackspace,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        match ctx.state.active_picker_kind {
            Some(PickerKind::Provider) => ctx.state.provider_picker.backspace(),
            None => {}
        }
        CommandAction::Continue
    }

    /// Confirms the active picker selection, dispatching to kind-specific logic.
    fn on_confirm(
        _cmd: &PickerConfirm,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        match ctx.state.active_picker_kind {
            Some(PickerKind::Provider) => Self::confirm_provider(ctx),
            None => {}
        }
        CommandAction::Continue
    }

    /// Moves the active picker selection up.
    fn on_move_up(
        _cmd: &PickerMoveUp,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        match ctx.state.active_picker_kind {
            Some(PickerKind::Provider) => ctx.state.provider_picker.move_up(PICKER_MAX_VISIBLE),
            None => {}
        }
        CommandAction::Continue
    }

    /// Moves the active picker selection down.
    fn on_move_down(
        _cmd: &PickerMoveDown,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        match ctx.state.active_picker_kind {
            Some(PickerKind::Provider) => ctx.state.provider_picker.move_down(PICKER_MAX_VISIBLE),
            None => {}
        }
        CommandAction::Continue
    }

    /// Moves the active picker filter cursor left.
    fn on_move_cursor_left(
        _cmd: &PickerMoveCursorLeft,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        match ctx.state.active_picker_kind {
            Some(PickerKind::Provider) => ctx.state.provider_picker.move_cursor_left(),
            None => {}
        }
        CommandAction::Continue
    }

    /// Moves the active picker filter cursor right.
    fn on_move_cursor_right(
        _cmd: &PickerMoveCursorRight,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) -> CommandAction {
        match ctx.state.active_picker_kind {
            Some(PickerKind::Provider) => ctx.state.provider_picker.move_cursor_right(),
            None => {}
        }
        CommandAction::Continue
    }

    /// Provider-specific confirm: switches provider and closes the picker.
    fn confirm_provider(ctx: &mut HandlerContext<'_, AppState, Services>) {
        let Some(entry) = ctx.state.provider_picker.selected_item() else {
            return;
        };
        if !entry.is_available {
            return;
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
    }
}

/// Maximum number of visible result rows used for scroll clamping in the handler.
/// The actual visible rows are determined dynamically by the renderer based on
/// terminal height. This value is a generous upper bound so the handler's scroll
/// offset tracking stays reasonable.
const PICKER_MAX_VISIBLE: usize = 100;

/// Loads provider entries into the picker state, ready for display.
///
/// Reads from the provider registry and model cache, applies available-first
/// sorting and active-provider promotion, then stores the entries via
/// [`SelectionState::set_items`].
pub fn load_provider_picker_items(services: &Services, state: &mut AppState) {
    let registry = services.provider_registry().read();
    let api_keys = services.api_keys().read();
    let all = load_provider_entries(&registry, &api_keys, state.model_cache.as_ref());
    let entries = sorted_entries(&all, "", &state.active_provider);
    state.provider_picker.set_items(entries);
}
