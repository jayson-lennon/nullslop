//! Shared protocol types for communication between the nullslop host and extensions.
//!
//! This crate defines the common language of commands, events, key representations,
//! interaction modes, and chat data that the host and all extensions agree on.
//! Every type here is serializable and travels across the extension boundary.
//!
//! Runtime-mutable state types ([`AppState`], [`ShutdownTracker`], [`ChatInputBoxState`])
//! live in `nullslop-component`.
//!
//! [`AppState`]: nullslop_component::AppState
//! [`ShutdownTracker`]: nullslop_component::ShutdownTracker
//! [`ChatInputBoxState`]: nullslop_component::ChatInputBoxState

pub mod action;
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
pub use chat::{ChatEntry, ChatEntryKind};
pub use command::Command;
pub use custom::{CommandMsg, EchoCommand, EventMsg};
pub use event::Event;
pub use key::{Key, KeyEvent, Modifiers};
pub use mode::Mode;
