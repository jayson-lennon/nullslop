//! Chat input box plugin: consolidated handler and UI element.
//!
//! This module provides a single [`ChatInputBoxHandler`] that handles all
//! chat input and mode-switching commands (previously split across
//! `InputModePlugin` and `NormalModePlugin`), and a [`ChatInputBoxElement`]
//! that renders the input box in the TUI.

pub mod element;
pub mod handler;

pub use element::ChatInputBoxElement;
pub(crate) use handler::ChatInputBoxHandler;
