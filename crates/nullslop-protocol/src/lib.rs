//! nullslop-protocol: shared types for the nullslop component system.
//!
//! This crate contains all command types, event types, wrapper enums,
//! [`Mode`], [`CommandAction`], and [`AppState`]. It is the single import
//! point for everything the component system needs. Both the TUI host and
//! extensions depend on this crate for wire-protocol types.

pub mod action;
pub mod app_state;
pub mod chat;
pub mod chat_input;
pub mod command;
pub mod custom;
pub mod event;
pub mod key;
pub mod mode;
pub mod shutdown;
pub mod system;

// Re-export primary types
pub use action::CommandAction;
pub use app_state::AppState;
pub use chat::{ChatEntry, ChatEntryKind};
pub use chat_input::ChatInputBoxState;
pub use command::Command;
pub use custom::{CommandMsg, EchoCommand, EventMsg};
pub use event::Event;
pub use key::{Key, KeyEvent, Modifiers};
pub use mode::Mode;
