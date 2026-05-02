//! Provider picker commands.

use serde::{Deserialize, Serialize};

use crate::CommandMsg;

/// Insert a character into the picker filter.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("provider_picker")]
pub struct PickerInsertChar {
    /// The character to insert into the filter.
    pub ch: char,
}

/// Delete the last character from the picker filter.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("provider_picker")]
pub struct PickerBackspace;

/// Confirm the current picker selection.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("provider_picker")]
pub struct PickerConfirm;

/// Move the picker selection up.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("provider_picker")]
pub struct PickerMoveUp;

/// Move the picker selection down.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("provider_picker")]
pub struct PickerMoveDown;

/// Move the picker filter cursor left.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("provider_picker")]
pub struct PickerMoveCursorLeft;

/// Move the picker filter cursor right.
#[derive(Debug, Clone, Serialize, Deserialize, CommandMsg)]
#[cmd("provider_picker")]
pub struct PickerMoveCursorRight;
