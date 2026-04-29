//! nullslop-core: application runtime for the nullslop agent harness.
//!
//! The processing loop ([`AppCore`]) receives messages, routes commands and events
//! through a component bus, and forwards them to the extension host. Shared state
//! ([`State`]) is accessible from any thread via read/write guards. The extension
//! host ([`ExtensionHost`]) manages discovery, lifecycle, and message routing for
//! extension processes.
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
