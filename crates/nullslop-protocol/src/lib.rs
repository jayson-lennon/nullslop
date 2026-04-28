//! nullslop-protocol: shared types for the nullslop plugin system.
//!
//! This crate contains all command types, event types, wrapper enums,
//! [`Mode`], [`CommandAction`], and [`AppState`]. It is the single import
//! point for everything the plugin system needs. Both the TUI host and
//! extensions depend on this crate for wire-protocol types.

pub mod action;
pub mod app_state;
pub mod chat;
pub mod chat_input_state;
pub mod command;
pub mod event;
pub mod key;
pub mod mode;

// Re-export primary types
pub use action::CommandAction;
pub use app_state::AppState;
pub use chat::{ChatEntry, ChatEntryKind};
pub use chat_input_state::ChatInputBoxState;
pub use command::Command;
pub use event::Event;
pub use key::{Key, KeyEvent, Modifiers};
pub use mode::Mode;
