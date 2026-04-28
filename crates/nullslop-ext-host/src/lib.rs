//! nullslop-ext-host: extension host implementation.
//!
//! Contains the process-based extension host that discovers, spawns,
//! and manages extension child processes. The [`ExtensionHost`] and [`ExtHostSender`]
//! traits live in `nullslop-core`; this crate provides the concrete implementation.

pub mod discovery;
pub mod fake;
pub mod host;
pub mod process;

pub use process::ProcessExtensionHost;
