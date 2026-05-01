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

#[cfg(test)]
mod tests {
    use super::ShutdownTrackerState;

    #[test]
    fn track_adds_to_pending() {
        // Given a new shutdown tracker.
        let mut tracker = ShutdownTrackerState::new();

        // When tracking an actor.
        tracker.track("ext-a");

        // Then the actor appears in pending names.
        assert_eq!(tracker.pending_names(), vec!["ext-a".to_string()]);
    }

    #[test]
    fn complete_removes_from_pending() {
        // Given a tracker with one tracked actor.
        let mut tracker = ShutdownTrackerState::new();
        tracker.track("ext-a");

        // When completing that actor.
        let was_tracked = tracker.complete("ext-a");

        // Then it was known and pending is now empty.
        assert!(was_tracked);
        assert!(tracker.pending_names().is_empty());
    }

    #[test]
    fn is_complete_false_when_not_active() {
        // Given a tracker with shutdown not active.
        let tracker = ShutdownTrackerState::new();

        // When checking is_complete.
        // Then it returns false because shutdown is not active.
        assert!(!tracker.is_complete());
    }

    #[test]
    fn is_complete_false_when_pending() {
        // Given a tracker with an active shutdown and one pending actor.
        let mut tracker = ShutdownTrackerState::new();
        tracker.track("ext-a");
        tracker.begin_shutdown();

        // When checking is_complete.
        // Then it returns false because an actor is still pending.
        assert!(!tracker.is_complete());
    }

    #[test]
    fn is_complete_true_when_active_and_empty() {
        // Given a tracker with an active shutdown and no pending actors.
        let mut tracker = ShutdownTrackerState::new();
        tracker.begin_shutdown();

        // When checking is_complete.
        // Then it returns true.
        assert!(tracker.is_complete());
    }

    #[test]
    fn pending_names_returns_pending() {
        // Given a tracker with three tracked actors.
        let mut tracker = ShutdownTrackerState::new();
        tracker.track("ext-a");
        tracker.track("ext-b");
        tracker.track("ext-c");

        // When collecting pending names.
        let mut names = tracker.pending_names();
        names.sort();

        // Then all three actors are listed.
        assert_eq!(names, vec!["ext-a", "ext-b", "ext-c"]);
    }
}
