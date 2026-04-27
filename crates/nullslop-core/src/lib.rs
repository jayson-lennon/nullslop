//! nullslop-core: shared types for the nullslop TUI agent harness.
//!
//! This crate re-exports types from `nullslop-protocol` (command/event system,
//! domain data, key types) and adds host-side concerns: thread-safe state
//! wrapper and extension registry.

pub mod app_data;
pub mod chat;
pub mod command;
pub mod event;
pub mod extension;
pub mod key;
pub mod state;

// Re-export primary types from nullslop-protocol
pub use app_data::AppData;
pub use chat::{ChatEntry, ChatEntryKind};
pub use command::Command;
pub use event::Event;
pub use extension::{ExtensionManifest, ExtensionRegistry, RegisteredExtension};
pub use key::{Key, KeyEvent, Modifiers};
pub use state::{State, StateReadGuard, StateWriteGuard};

// Re-export new protocol types
pub use nullslop_protocol::{CommandAction, Mode};
