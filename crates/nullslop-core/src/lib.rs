//! nullslop-core: shared types for the nullslop TUI agent harness.
//!
//! This crate contains the command/event system, domain data,
//! key types, and extension protocol types shared between the TUI
//! host and extension processes.

pub mod app_data;
pub mod chat;
pub mod command;
pub mod event;
pub mod extension;
pub mod key;
pub mod state;

// Re-export primary types
pub use app_data::AppData;
pub use chat::{ChatEntry, ChatEntryKind};
pub use command::Command;
pub use event::Event;
pub use extension::{ExtensionManifest, ExtensionRegistry, RegisteredExtension};
pub use key::{Key, KeyEvent, Modifiers};
pub use state::{State, StateReadGuard, StateWriteGuard};
