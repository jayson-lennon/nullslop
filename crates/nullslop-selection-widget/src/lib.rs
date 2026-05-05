//! Reusable search+filter+select widget for ratatui TUI applications.
//!
//! This crate provides a [`PickerItem`] trait and a [`SelectionState`] generic state machine.
//! Consumers bring their own item types, define how items display and render, and the widget
//! handles fuzzy filtering, cursor navigation, and selection management.
//!
//! Commands, handlers, and keymap wiring live in consumer crates — this crate is purely
//! the state and types layer.

pub mod item;
pub mod state;
pub mod widget;

pub use item::PickerItem;
pub use state::SelectionState;
pub use widget::{
    PICKER_H_PAD_FRAC, PICKER_MAX_HEIGHT_FRAC, PICKER_MIN_WIDTH, SelectionWidget,
    compute_popup_rect,
};
