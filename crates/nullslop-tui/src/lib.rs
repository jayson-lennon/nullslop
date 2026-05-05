//! nullslop-tui: terminal user interface for the nullslop agent harness.
//!
//! This crate provides the main event loop, terminal setup, rendering,
//! and key handling for the nullslop TUI application.

pub mod app;
pub mod app_state;
pub mod config;
pub mod convert;
pub mod keymap;
pub mod msg;
pub mod render;
pub mod run;
pub mod scope;
pub mod selection;
pub mod suspend;
pub mod terminal;

pub use app::TuiApp;
pub use app_state::AppStatus;
pub use keymap::KeyCategory;
pub use msg::handler::MsgHandler;
pub use nullslop_core::{ActorHost, ActorHostService, AppCore, AppMsg};
pub use nullslop_services::Services;
pub use run::{TuiRunError, run};
pub use scope::Scope;
