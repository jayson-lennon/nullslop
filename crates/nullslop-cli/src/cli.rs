//! CLI argument definitions using clap derive.

use clap::{Parser, Subcommand};

/// nullslop — a TUI agent harness with a component/extension system.
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
    Headless {
        /// Headless subcommand.
        #[command(subcommand)]
        command: Option<HeadlessCommands>,
    },
}

/// Headless subcommands.
#[derive(Debug, Subcommand)]
pub enum HeadlessCommands {
    /// Send a chat message.
    SendChat {
        /// The message text to send.
        message: String,
    },
    /// Run a keystroke script.
    Script {
        /// Path to a script file with one key sequence per line.
        path: String,
    },
}
