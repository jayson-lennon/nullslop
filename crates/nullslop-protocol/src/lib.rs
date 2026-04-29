//! nullslop-protocol: wire-protocol types for the nullslop component system.
//!
//! This crate contains all command types, event types, wrapper enums,
//! [`Mode`], [`CommandAction`], [`ChatEntry`], and [`ChatEntryKind`].
//! It is the single import point for wire-protocol types that get
//! serialized and transmitted between host and extensions.
//!
//! Runtime-mutable state types ([`AppState`], [`ShutdownTracker`], [`ChatInputBoxState`])
//! live in `nullslop-component-core`.
//!
//! [`AppState`]: nullslop_component_core::AppState
//! [`ShutdownTracker`]: nullslop_component_core::ShutdownTracker
//! [`ChatInputBoxState`]: nullslop_component_core::ChatInputBoxState

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
