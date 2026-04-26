//! nullslop-tui: terminal user interface for the nullslop agent harness.
//!
//! This crate provides the main event loop, terminal setup, rendering,
//! and key handling for the nullslop TUI application.

pub mod app;
pub mod app_state;
pub mod command;
pub mod convert;
pub mod ext;
pub mod keymap;
pub mod msg;
pub mod render;
pub mod run;
pub mod scope;
pub mod services;
pub mod suspend;
pub mod terminal;
pub mod tui_state;

pub use app::TuiApp;
pub use app_state::AppStatus;
pub use command::TuiCommand;
pub use ext::{ExtensionHost, ExtensionHostService};
pub use keymap::KeyCategory;
pub use msg::handler::MsgHandler;
pub use run::{TuiRunError, run};
pub use scope::Scope;
pub use suspend::{Suspend, SuspendAction};
pub use tui_state::TuiState;
