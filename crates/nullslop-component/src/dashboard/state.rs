//! Dashboard state — tracks actor names, descriptions, and their startup status.
//!
//! Each actor goes through a lifecycle: `Starting` → `Running`.
//! The dashboard state records the current status and description for display.

use std::collections::HashMap;

/// The startup status of an actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorStatus {
    /// The actor is currently starting up.
    Starting,
    /// The actor has finished starting and is ready.
    Running,
}

/// A tracked actor's display data.
#[derive(Debug, Clone)]
pub struct ActorEntry {
    /// The actor's display name.
    pub name: String,
    /// A short description of what the actor does.
    pub description: Option<String>,
    /// The actor's current lifecycle status.
    pub status: ActorStatus,
}

/// Tracks the startup status of all actors.
#[derive(Debug, Clone, Default)]
pub struct DashboardState {
    /// Actor name → entry data.
    actors: HashMap<String, ActorEntry>,
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
    pub fn mark_starting<S>(&mut self, name: S, description: Option<String>)
    where
        S: AsRef<str>,
    {
        let name = name.as_ref();
        let is_new = !self.actors.contains_key(name);
        if is_new {
            self.order.push(name.to_owned());
        }
        let entry = self.actors.get_mut(name);
        match entry {
            Some(e) => {
                e.status = ActorStatus::Starting;
                if description.is_some() {
                    e.description = description;
                }
            }
            None => {
                self.actors.insert(
                    name.to_owned(),
                    ActorEntry {
                        name: name.to_owned(),
                        description,
                        status: ActorStatus::Starting,
                    },
                );
            }
        }
    }

    /// Record that an actor is now running.
    ///
    /// If the actor was not previously tracked (no `mark_starting` call),
    /// it is added with `Running` status.
    pub fn mark_running<S>(&mut self, name: S, description: Option<String>)
    where
        S: AsRef<str>,
    {
        let name = name.as_ref();
        let is_new = !self.actors.contains_key(name);
        if is_new {
            self.order.push(name.to_owned());
        }
        let entry = self.actors.get_mut(name);
        match entry {
            Some(e) => {
                e.status = ActorStatus::Running;
                if description.is_some() {
                    e.description = description;
                }
            }
            None => {
                self.actors.insert(
                    name.to_owned(),
                    ActorEntry {
                        name: name.to_owned(),
                        description,
                        status: ActorStatus::Running,
                    },
                );
            }
        }
    }

    /// Returns all tracked actors in insertion order.
    #[must_use]
    pub fn actors(&self) -> Vec<&ActorEntry> {
        self.order
            .iter()
            .filter_map(|name| self.actors.get(name))
            .collect()
    }
}
