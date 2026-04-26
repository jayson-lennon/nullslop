//! CLI argument definitions using clap derive.

use clap::{Parser, Subcommand};

/// nullslop — a TUI agent harness with a plugin/extension system.
#[derive(Debug, Parser)]
#[command(name = "nullslop", version, about)]
pub struct Cli {
    /// The subcommand to run. If omitted, launches the TUI.
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Available subcommands.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Launch the TUI (default when no subcommand is given).
    Tui,

    /// Run without a terminal interface.
    Headless,
}
