//! nullslop-core: shared types for the nullslop TUI agent harness.
//!
//! This crate provides host-side concerns: thread-safe state wrapper,
//! extension host trait and registry, and application core.
//!
//! Protocol types (commands, events, keys, chat entries) are in `nullslop-protocol`.
//! Import them directly from there.

pub mod app_core;
pub mod app_msg;
pub mod ext_host;
pub mod extension;
pub mod state;

// Re-export primary types owned by this crate
pub use app_core::{AppCore, TickResult};
pub use app_msg::AppMsg;
pub use ext_host::{ExtHostSender, ExtensionError, ExtensionHost, ExtensionHostService};
pub use extension::{ExtensionManifest, ExtensionRegistry, RegisteredExtension};
pub use state::{State, StateReadGuard, StateWriteGuard};

// Re-export protocol types for convenience (types that don't have a module shadowing them)
pub use nullslop_protocol::{CommandAction, Mode};
