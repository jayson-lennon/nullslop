//! jinn-tui: terminal user interface for the jinn agent harness.
//!
//! This crate provides the main event loop, terminal setup, rendering,
//! and key handling for the jinn TUI application.

pub mod app;
pub mod app_state;
pub mod config;
pub mod convert;
pub mod keymap;
pub mod keymap_gen;
pub mod launch;
pub mod msg;
#[cfg(test)]
mod nav_e2e_test;
pub mod render;
pub mod run;
pub mod scope;
pub mod selection;
pub mod suspend;
pub mod terminal;

pub use app::TuiApp;
pub use app::TuiAppBuilder;
pub use app_state::AppStatus;
pub use jinn_domain::AppCore;
pub use jinn_domain::Services;
pub use keymap::KeyCategory;
pub use launch::{LaunchError, launch, load_compaction_prompt, load_theme};
pub use msg::handler::MsgHandler;
pub use run::{TuiRunError, run};
pub use scope::Scope;

#[cfg(test)]
mod app_tests;

#[cfg(test)]
mod render_tests;

#[cfg(test)]
mod selection_tests;
