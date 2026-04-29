//! nullslop-component-ui: renderable UI elements for the component system.
//!
//! This crate provides the [`UiElement`] trait and [`UiRegistry`] for composable,
//! component-driven rendering. It bridges `nullslop-component-core` and `ratatui`,
//! allowing components to register UI elements that the TUI render loop draws.
//!
//! # Two-struct pattern
//!
//! Handlers and elements are separate structs that communicate through
//! [`AppState`]:
//!
//! - **Handlers** implement [`CommandHandler`](nullslop_component_core::CommandHandler)
//!   or [`EventHandler`](nullslop_component_core::EventHandler) and mutate state
//!   during command/event processing.
//! - **Elements** implement [`UiElement`] and read state during rendering.
//!
//! No shared instances, no `Arc`, no `RefCell` for the common case. If a handler
//! and element genuinely need shared internal state, they set it up explicitly.
//!
//! # Architecture
//!
//! ```text
//! nullslop-component-core     (bus, Handler traits)
//!       │
//!       ▼
//! nullslop-component-ui       (UiElement trait + UiRegistry)
//!       │
//!       ▼
//! nullslop-component          (built-in components implement UiElement)
//!       │
//!       ▼
//! nullslop-tui             (discovers UiElements via registry, renders them)
//! ```

pub mod element;
pub mod fake;
pub mod registry;

pub use element::UiElement;
pub use registry::UiRegistry;
