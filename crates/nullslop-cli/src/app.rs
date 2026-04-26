//! Top-level application state.
//!
//! [`App`] holds memory-resident resources that live for the entire
//! process lifetime — the tokio runtime, configuration, and any
//! other long-lived services. It is owned by the main thread.

use error_stack::{Report, ResultExt};
use tokio::runtime::Runtime;
use wherror::Error;

/// Error type for top-level application initialization.
#[derive(Debug, Error)]
#[error(debug)]
pub struct AppError;

/// Top-level application state.
///
/// Created once in [`main`](crate::main) and passed to whichever
/// runner (TUI, headless, etc.) handles the command. Fully owned
/// by the main thread — no `Arc` needed.
pub struct App {
    /// The tokio runtime. Kept alive here; pass [`Handle`](tokio::runtime::Handle)s
    /// to anything that needs to spawn tasks.
    runtime: Runtime,
}

impl App {
    /// Creates a new top-level app with a default multi-threaded runtime.
    ///
    /// # Errors
    ///
    /// Returns an error if the tokio runtime cannot be created.
    pub fn new() -> Result<Self, Report<AppError>> {
        let runtime = Runtime::new()
            .change_context(AppError)
            .attach("failed to create tokio runtime")?;
        Ok(Self { runtime })
    }

    /// Returns a handle to the tokio runtime for spawning tasks.
    #[must_use]
    pub fn handle(&self) -> tokio::runtime::Handle {
        self.runtime.handle().clone()
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new().expect("failed to create default App")
    }
}
