//! nullslop-cli: command-line interface definitions for the nullslop agent harness.
//!
//! Provides CLI argument parsing via [`Cli`]. The actual dispatch and running
//! is handled by the root crate.

pub mod cli;

pub use cli::Cli;
