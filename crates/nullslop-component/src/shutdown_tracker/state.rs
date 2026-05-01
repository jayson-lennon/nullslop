//! Bookkeeping for actors still running during shutdown.
//!
//! Tracks which actors have started and whether shutdown has been triggered,
//! so the application can wait for all of them to finish before exiting.

use std::collections::HashSet;

/// Tracks which actors are still active during a shutdown.
#[derive(Debug, Clone, Default)]
pub struct ShutdownTrackerState {
    /// Actors that are currently running.
    pending: HashSet<String>,
    /// Whether the application has begun shutting down.
    shutdown_active: bool,
}

impl ShutdownTrackerState {
    /// Create a tracker with no actors and shutdown inactive.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Signal that the application has started shutting down.
    pub fn begin_shutdown(&mut self) {
        self.shutdown_active = true;
    }

    /// Record that an actor has started.
    pub fn track(&mut self, name: &str) {
        self.pending.insert(name.to_string());
    }

    /// Record that an actor has finished shutting down.
    ///
    /// Returns `true` if this actor was known to be running.
    pub fn complete(&mut self, name: &str) -> bool {
        self.pending.remove(name)
    }

    /// Returns `true` when shutdown is in progress and every actor has finished.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.shutdown_active && self.pending.is_empty()
    }

    /// Returns the names of actors that are still running.
    #[must_use]
    pub fn pending_names(&self) -> Vec<String> {
        self.pending.iter().cloned().collect()
    }
}
