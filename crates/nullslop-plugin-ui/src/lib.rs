//! nullslop-plugin-ui: renderable UI elements for the plugin system.
//!
//! This crate provides the [`UiElement`] trait and [`UiRegistry`] for composable,
//! plugin-driven rendering. It bridges `nullslop-plugin-core` and `ratatui`,
//! allowing plugins to register UI elements that the TUI render loop draws.
//!
//! # Two-struct pattern
//!
//! Handlers and elements are separate structs that communicate through
//! [`AppState`]:
//!
//! - **Handlers** implement [`CommandHandler`](nullslop_plugin_core::CommandHandler)
//!   or [`EventHandler`](nullslop_plugin_core::EventHandler) and mutate state
//!   during command/event processing.
//! - **Elements** implement [`UiElement`] and read state during rendering.
//!
//! No shared instances, no `Arc`, no `RefCell` for the common case. If a handler
//! and element genuinely need shared internal state, they set it up explicitly.
//!
//! # Architecture
//!
//! ```text
//! nullslop-plugin-core     (bus, Handler traits)
//!       │
//!       ▼
//! nullslop-plugin-ui       (UiElement trait + UiRegistry)
//!       │
//!       ▼
//! nullslop-plugin          (built-in plugins implement UiElement)
//!       │
//!       ▼
//! nullslop-tui             (discovers UiElements via registry, renders them)
//! ```

pub mod element;
pub mod fake;
pub mod registry;

pub use element::UiElement;
pub use registry::UiRegistry;
