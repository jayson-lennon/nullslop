//! Dashboard state — tracks actor names, descriptions, their startup status, and selection.
//!
//! Each actor goes through a lifecycle: `Starting` → `Running`.
//! The dashboard state records the current status and description for display.
//! The user can scroll through entries with `j`/`k`, which moves the selection
//! indicator and scrolls the view to keep the selected entry visible.

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
    /// Index of the currently selected actor entry.
    selected_index: usize,
    /// Vertical scroll offset in visual lines.
    scroll_offset: u16,
}

impl DashboardState {
    /// Create an empty dashboard state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the index of the currently selected actor entry.
    #[must_use]
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// Returns the current vertical scroll offset in visual lines.
    #[must_use]
    pub fn scroll_offset(&self) -> u16 {
        self.scroll_offset
    }

    /// Moves the selection to the next actor entry.
    ///
    /// Clamps at the last entry — does nothing if already at the end.
    pub fn select_next(&mut self) {
        let count = self.order.len();
        if count > 0 && self.selected_index < count - 1 {
            self.selected_index += 1;
        }
    }

    /// Moves the selection to the previous actor entry.
    ///
    /// Clamps at the first entry — does nothing if already at the beginning.
    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_next_increments_index() {
        // Given 3 actors with selection at index 0.
        let mut state = DashboardState::new();
        state.mark_starting("a", None);
        state.mark_starting("b", None);
        state.mark_starting("c", None);

        // When selecting next.
        state.select_next();

        // Then the selected index is 1.
        assert_eq!(state.selected_index(), 1);
    }

    #[test]
    fn select_next_clamps_at_last() {
        // Given 3 actors with selection at index 2.
        let mut state = DashboardState::new();
        state.mark_starting("a", None);
        state.mark_starting("b", None);
        state.mark_starting("c", None);
        state.select_next();
        state.select_next();
        assert_eq!(state.selected_index(), 2);

        // When selecting next.
        state.select_next();

        // Then the index stays at 2.
        assert_eq!(state.selected_index(), 2);
    }

    #[test]
    fn select_prev_decrements_index() {
        // Given 3 actors with selection at index 1.
        let mut state = DashboardState::new();
        state.mark_starting("a", None);
        state.mark_starting("b", None);
        state.mark_starting("c", None);
        state.select_next();

        // When selecting previous.
        state.select_prev();

        // Then the selected index is 0.
        assert_eq!(state.selected_index(), 0);
    }

    #[test]
    fn select_prev_clamps_at_zero() {
        // Given 2 actors with selection at index 0.
        let mut state = DashboardState::new();
        state.mark_starting("a", None);
        state.mark_starting("b", None);

        // When selecting previous.
        state.select_prev();

        // Then the index stays at 0.
        assert_eq!(state.selected_index(), 0);
    }

    #[test]
    fn select_next_noop_with_no_actors() {
        // Given an empty dashboard.
        let mut state = DashboardState::new();

        // When selecting next.
        state.select_next();

        // Then the index stays at 0.
        assert_eq!(state.selected_index(), 0);
    }
}
