//! nullslop: a TUI agent harness with a component/actor system.

pub mod app;
pub mod headless;
pub mod runner;
pub mod tracing;

pub use app::{App, AppError};
pub use headless::HeadlessApp;
pub use runner::Runner;
