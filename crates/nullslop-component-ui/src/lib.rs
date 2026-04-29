//! Rendering layer for the component system.
//!
//! This crate defines [`UiElement`] — the trait for drawable UI components —
//! and [`UiRegistry`] — the collection that holds them for the render loop.
//! Components register elements during startup, and the TUI layer iterates
//! them each frame.
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
//! Handlers and elements do not share instances. If a handler and element
//! genuinely need shared internal state, they set it up explicitly.
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
