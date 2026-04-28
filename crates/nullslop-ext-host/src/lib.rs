//! nullslop-ext-host: extension host implementation.
//!
//! Contains the process-based extension host that discovers, spawns,
//! and manages extension child processes, and the in-memory extension
//! host that runs extensions as OS threads without serialization.
//! The [`ExtensionHost`] and [`ExtHostSender`] traits live in `nullslop-core`;
//! this crate provides concrete implementations.

pub mod discovery;
pub mod fake;
pub mod host;
pub mod in_memory;
pub mod process;

pub use in_memory::InMemoryExtensionHost;
pub use process::ProcessExtensionHost;
