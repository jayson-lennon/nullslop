//! Shared protocol types for communication between the nullslop host and actors.
//!
//! This crate defines the common language of commands, events, key representations,
//! interaction modes, and chat data that the host and all actors agree on.
//! Every type here is serializable and travels across the actor boundary.
//!
//! Runtime-mutable state types ([`AppState`], [`ShutdownTracker`], [`ChatInputBoxState`])
//! live in `nullslop-component`.
//!
//! [`AppState`]: nullslop_component::AppState
//! [`ShutdownTracker`]: nullslop_component::ShutdownTracker
//! [`ChatInputBoxState`]: nullslop_component::ChatInputBoxState

pub mod action;
pub mod actor;
pub mod actor_name;
pub mod chat;
pub mod chat_input;
pub mod command;
pub mod custom;
pub mod event;
pub mod key;
pub mod mode;
pub mod provider;
pub mod session;
pub mod system;
pub mod tab;

// Re-export primary types
pub use action::CommandAction;
pub use actor::{ActorShutdownCompleted, ActorStarted, ActorStarting};
pub use actor_name::ActorName;
pub use chat::{ChatEntry, ChatEntryKind};
pub use command::Command;
pub use custom::{CommandMsg, CommandName, EventMsg, EventTypeName};
pub use event::Event;
pub use key::{Key, KeyEvent, Modifiers};
pub use mode::Mode;
pub use nullslop_protocol_derive::{CommandMsg, EventMsg};
pub use provider::entries_to_messages;
pub use provider::{LlmMessage, LlmRole};
pub use session::SessionId;
pub use tab::ActiveTab;
pub use tab::TabDirection;
