//! Dashboard state — tracks actor names and their startup status.
//!
//! Each actor goes through a lifecycle: `Starting` → `Started`.
//! The dashboard state records the current status for display.

use std::collections::HashMap;

/// The startup status of an actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorStatus {
    /// The actor is currently starting up.
    Starting,
    /// The actor has finished starting and is ready.
    Started,
}

/// Tracks the startup status of all actors.
#[derive(Debug, Clone, Default)]
pub struct DashboardState {
    /// Actor name → current status.
    actors: HashMap<String, ActorStatus>,
    /// Insertion-order keys for stable display.
    order: Vec<String>,
}

impl DashboardState {
    /// Create an empty dashboard state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that an actor has started the startup process.
    pub fn mark_starting(&mut self, name: &str) {
        if !self.actors.contains_key(name) {
            self.order.push(name.to_string());
        }
        self.actors.insert(name.to_string(), ActorStatus::Starting);
    }

    /// Record that an actor has finished starting.
    ///
    /// If the actor was not previously tracked (no `mark_starting` call),
    /// it is added with `Started` status.
    pub fn mark_started(&mut self, name: &str) {
        if !self.actors.contains_key(name) {
            self.order.push(name.to_string());
        }
        self.actors.insert(name.to_string(), ActorStatus::Started);
    }

    /// Returns all tracked actors in insertion order with their status.
    #[must_use]
    pub fn actors(&self) -> Vec<(&str, ActorStatus)> {
        self.order
            .iter()
            .filter_map(|name| self.actors.get(name).map(|&status| (name.as_str(), status)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mark_starting_then_started() {
        // Given an empty dashboard.
        let mut dashboard = DashboardState::new();

        // When marking "actor-a" as starting, then started.
        dashboard.mark_starting("actor-a");
        dashboard.mark_started("actor-a");

        // Then "actor-a" has Started status.
        let actors = dashboard.actors();
        assert_eq!(actors.len(), 1);
        assert_eq!(actors[0], ("actor-a", ActorStatus::Started));
    }

    #[test]
    fn mark_started_without_starting() {
        // Given an empty dashboard.
        let mut dashboard = DashboardState::new();

        // When marking "actor-a" as started without prior starting.
        dashboard.mark_started("actor-a");

        // Then "actor-a" is tracked with Started status.
        let actors = dashboard.actors();
        assert_eq!(actors.len(), 1);
        assert_eq!(actors[0], ("actor-a", ActorStatus::Started));
    }

    #[test]
    fn actors_preserve_insertion_order() {
        // Given an empty dashboard.
        let mut dashboard = DashboardState::new();

        // When adding multiple actors.
        dashboard.mark_starting("alpha");
        dashboard.mark_starting("beta");
        dashboard.mark_started("beta");
        dashboard.mark_started("alpha");

        // Then order reflects first-seen order.
        let actors = dashboard.actors();
        assert_eq!(actors.len(), 2);
        assert_eq!(actors[0].0, "alpha");
        assert_eq!(actors[0].1, ActorStatus::Started);
        assert_eq!(actors[1].0, "beta");
        assert_eq!(actors[1].1, ActorStatus::Started);
    }

    #[test]
    fn empty_dashboard_has_no_actors() {
        // Given an empty dashboard.
        let dashboard = DashboardState::new();

        // Then there are no actors.
        assert!(dashboard.actors().is_empty());
    }
}
