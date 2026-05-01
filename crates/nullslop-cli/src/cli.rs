//! Command-line interface argument definitions.

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use clap_verbosity_flag::{Verbosity, WarnLevel};

/// nullslop — a TUI agent harness with a component/actor system.
#[derive(Debug, Parser)]
#[command(name = "nullslop", version, about)]
pub struct Cli {
    /// Verbosity level for logging.
    #[command(flatten)]
    pub verbosity: Verbosity<WarnLevel>,

    /// Directory for log file output (TUI mode). Defaults to current directory.
    #[arg(long)]
    pub log_dir: Option<PathBuf>,

    /// Use the sample LLM provider instead of a real backend.
    ///
    /// No API key is required. The provider responds to `!response`
    /// and `!think` commands with canned, streamed output.
    #[arg(long)]
    pub fake_llm: bool,

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
        /// Also log to a file in headless mode.
        #[arg(long)]
        log_file: Option<PathBuf>,

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
