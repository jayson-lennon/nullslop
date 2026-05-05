//! Picker kind — identifies which picker is currently active.
//!
//! A single set of `Picker*` commands, `Mode::Picker`, `Scope::Picker`,
//! and keymap bindings serve all pickers. [`PickerKind`] determines which
//! [`SelectionState`](nullslop_selection_widget::SelectionState) the commands
//! operate on.

use serde::{Deserialize, Serialize};

/// Which picker is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PickerKind {
    /// Provider/model picker.
    Provider,
}
