//! nullslop: a TUI agent harness with a component/extension system.

pub mod app;
pub mod headless;
pub mod runner;

pub use app::{App, AppError};
pub use headless::HeadlessApp;
pub use runner::Runner;
