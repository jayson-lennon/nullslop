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
    pub fn mark_starting<S>(&mut self, name: S)
    where
        S: AsRef<str>,
    {
        let name = name.as_ref();
        if !self.actors.contains_key(name) {
            self.order.push(name.to_owned());
        }
        self.actors.insert(name.to_owned(), ActorStatus::Starting);
    }

    /// Record that an actor has finished starting.
    ///
    /// If the actor was not previously tracked (no `mark_starting` call),
    /// it is added with `Started` status.
    pub fn mark_started<S>(&mut self, name: S)
    where
        S: AsRef<str>,
    {
        let name = name.as_ref();
        if !self.actors.contains_key(name) {
            self.order.push(name.to_owned());
        }
        self.actors.insert(name.to_owned(), ActorStatus::Started);
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
