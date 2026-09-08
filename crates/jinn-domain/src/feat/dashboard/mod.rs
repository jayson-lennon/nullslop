//! Dashboard - displays registered actors with lifecycle and service status.
//!
//! The dashboard is a full-screen tab showing one line per actor in the
//! system. Each entry has three independent pieces of information:
//!
//! - **Name + description** — the actor's identity and what it does.
//! - **Lifecycle** — generic `Starting` / `Running` / `Dead`, driven by the
//!   existing bus events (`ActorStarting`, `ActorStarted`,
//!   `ActorShutdownCompleted`).
//! - **Status message** — an optional free-form third column. For the discord
//!   bot this is the connection sub-state (`Connected`, `Disconnected`, error
//!   text). Other actors leave it `None` until they gain their own
//!   service-level reporting.
//!
//! [`DashboardActor`] owns the dashboard's slice cell (registered under
//! `dashboard:status` in the [`Slices`](crate::common::slices::Slices)
//! facade). It subscribes to the generic lifecycle events, to
//! [`DiscordStatusUpdate`] (republished by [`DiscordStatusActor`] from
//! the gateway kanal channel), and to [`DashboardNav`] for keyboard
//! navigation.
pub mod dashboard_actor;
pub mod key_routes;
pub mod nav;
pub mod status_actor;
pub mod view;

pub use dashboard_actor::{DashboardActor, DashboardActorDeps};
pub use key_routes::attach_dashboard_rows;
pub use nav::DashboardNav;
pub use status_actor::{DiscordStatusActor, DiscordStatusActorDeps, DiscordStatusUpdate};
use std::collections::HashMap;
pub use view::DashboardView;

/// The dashboard slice's slot in the [`Slices`](crate::common::slices::Slices)
/// facade.
///
/// Canonical key shared by actor wiring (which mints the cell), the
/// renderer (which resolves a read handle), and tests.
pub fn dashboard_slot() -> crate::common::slices::SlotKey {
    crate::common::slices::SlotKey::builtin("dashboard", "status")
}

use crate::common::AppUiRegistry;

/// Generic actor lifecycle, applicable to every actor in the system.
///
/// Re-exported from `jinn-core-types` (where the type now lives) so the
/// dashboard's historical import path keeps working during extraction.
pub use jinn_core_types::ActorLifecycle;

/// A single actor's display data in the dashboard.
#[derive(Debug, Clone)]
pub struct DashboardEntry {
    /// The actor's display name (also its unique key).
    pub name: String,
    /// A short description of what the actor does.
    pub description: Option<String>,
    /// The actor's current lifecycle phase.
    pub lifecycle: ActorLifecycle,
    /// Free-form third column. Discord writes its connection status here;
    /// other actors leave this `None`.
    pub status_message: Option<String>,
}

/// Tracks the status of all actors for dashboard display.
///
/// Owned by [`crate::feat::dashboard::dashboard_actor::DashboardActor`] via
/// `frontend.dashboard`. The actor owns this field.
#[derive(Debug, Clone, Default)]
pub struct DashboardState {
    /// Actor name → entry data.
    actors: HashMap<String, DashboardEntry>,
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

    /// Clamps `scroll_offset` so the selected entry is always visible within
    /// a viewport of `viewport_height` rows.
    pub fn clamp_scroll(&mut self, viewport_height: u16) {
        if viewport_height == 0 {
            return;
        }
        let count = self.order.len() as u16;
        if count == 0 {
            self.scroll_offset = 0;
            return;
        }
        let sel = u16::try_from(self.selected_index).unwrap_or(u16::MAX);
        let bottom = self
            .scroll_offset
            .saturating_add(viewport_height)
            .saturating_sub(1);
        match sel {
            s if s < self.scroll_offset => {
                self.scroll_offset = s;
            }
            s if s > bottom => {
                self.scroll_offset = s.saturating_sub(viewport_height).saturating_add(1);
            }
            _ => {}
        }
        let max_offset = count.saturating_sub(viewport_height);
        self.scroll_offset = self.scroll_offset.min(max_offset);
    }

    /// Returns all tracked actors in insertion order.
    #[must_use]
    pub fn actors(&self) -> Vec<&DashboardEntry> {
        self.order
            .iter()
            .filter_map(|name| self.actors.get(name))
            .collect()
    }

    /// Moves the selection to the next actor entry.
    ///
    /// Clamps at the last entry - does nothing if already at the end.
    pub fn select_next(&mut self) {
        let count = self.order.len();
        if count > 0 && self.selected_index < count - 1 {
            self.selected_index += 1;
        }
    }

    /// Moves the selection to the previous actor entry.
    ///
    /// Clamps at the first entry - does nothing if already at the beginning.
    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Moves the selection to the first actor entry.
    ///
    /// No-op if there are no actors.
    pub fn select_first(&mut self) {
        if !self.order.is_empty() {
            self.selected_index = 0;
        }
    }

    /// Moves the selection to the last actor entry.
    ///
    /// No-op if there are no actors.
    pub fn select_last(&mut self) {
        if !self.order.is_empty() {
            self.selected_index = self.order.len() - 1;
        }
    }

    /// Record that an actor is in (or has returned to) the startup phase.
    ///
    /// If the actor is new it is appended to the display order. Existing
    /// entries keep their description unless a new one is supplied.
    /// Resets the grid to empty — no actors, selection at top, scroll 0.
    pub fn clear(&mut self) {
        self.actors.clear();
        self.order.clear();
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    /// Record that an actor is in (or has returned to) the startup phase.
    ///
    /// If the actor is new it is appended to the display order. Existing
    /// entries keep their description unless a new one is supplied.
    pub fn mark_starting<S>(&mut self, name: S, description: Option<String>)
    where
        S: AsRef<str>,
    {
        self.upsert(name, description, ActorLifecycle::Starting);
    }

    /// Record that an actor has finished starting and is running.
    pub fn mark_running<S>(&mut self, name: S, description: Option<String>)
    where
        S: AsRef<str>,
    {
        self.upsert(name, description, ActorLifecycle::Running);
    }

    /// Record that an actor has shut down (intentionally or via crash).
    pub fn mark_dead<S>(&mut self, name: S, description: Option<String>)
    where
        S: AsRef<str>,
    {
        self.upsert(name, description, ActorLifecycle::Dead);
    }

    /// Update only the free-form status message for an actor, leaving its
    /// lifecycle untouched.
    ///
    /// Creates the entry (as `Starting`) if it does not already exist, so the
    /// gateway can report a connection status before the corresponding
    /// `ActorStarting` bus event arrives.
    pub fn set_status_message<S>(&mut self, name: S, message: Option<String>)
    where
        S: AsRef<str>,
    {
        let name = name.as_ref();
        if !self.actors.contains_key(name) {
            self.order.push(name.to_owned());
            self.actors.insert(
                name.to_owned(),
                DashboardEntry {
                    name: name.to_owned(),
                    description: None,
                    lifecycle: ActorLifecycle::Starting,
                    status_message: message,
                },
            );
            return;
        }
        if let Some(entry) = self.actors.get_mut(name) {
            entry.status_message = message;
        }
    }

    /// Insert-or-update helper applying a new lifecycle and optional
    /// description. Does not touch `status_message` on existing entries.
    fn upsert<S>(&mut self, name: S, description: Option<String>, lifecycle: ActorLifecycle)
    where
        S: AsRef<str>,
    {
        let name = name.as_ref();
        let is_new = !self.actors.contains_key(name);
        if is_new {
            self.order.push(name.to_owned());
            self.actors.insert(
                name.to_owned(),
                DashboardEntry {
                    name: name.to_owned(),
                    description,
                    lifecycle,
                    status_message: None,
                },
            );
            return;
        }
        if let Some(entry) = self.actors.get_mut(name) {
            entry.lifecycle = lifecycle;
            if description.is_some() {
                entry.description = description;
            }
        }
    }
}

/// Register dashboard UI element.
///
/// No-op placeholder until the `DashboardElement` renderer lands in a later
/// phase; kept so callers (`actor_wiring` / feature registration) can wire it
/// without a second touch.
pub fn register(_registry: &mut AppUiRegistry) {}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable,
        clippy::indexing_slicing,
        reason = "test code"
    )]
    use super::*;

    fn entry<'a>(state: &'a DashboardState, name: &str) -> &'a DashboardEntry {
        state.actors.get(name).expect("entry should exist")
    }

    #[rstest::rstest]
    fn mark_starting_creates_entry_with_starting_lifecycle() {
        // Given an empty dashboard.
        let mut state = DashboardState::new();

        // When marking an actor as starting.
        state.mark_starting("echo", None);

        // Then the entry exists with Starting lifecycle.
        assert_eq!(entry(&state, "echo").lifecycle, ActorLifecycle::Starting);
    }

    #[rstest::rstest]
    fn mark_running_creates_entry_with_running_lifecycle() {
        // Given an empty dashboard.
        let mut state = DashboardState::new();

        // When marking an actor as running directly.
        state.mark_running("echo", None);

        // Then the entry exists with Running lifecycle.
        assert_eq!(entry(&state, "echo").lifecycle, ActorLifecycle::Running);
    }

    #[rstest::rstest]
    fn mark_running_transitions_existing_starting_to_running() {
        // Given a dashboard with a starting actor.
        let mut state = DashboardState::new();
        state.mark_starting("echo", None);

        // When marking the same actor as running.
        state.mark_running("echo", None);

        // Then the lifecycle transitions to Running.
        assert_eq!(entry(&state, "echo").lifecycle, ActorLifecycle::Running);
    }

    #[rstest::rstest]
    fn mark_dead_transitions_to_dead() {
        // Given a dashboard with a running actor.
        let mut state = DashboardState::new();
        state.mark_running("echo", None);

        // When marking the actor as dead.
        state.mark_dead("echo", None);

        // Then the lifecycle transitions to Dead.
        assert_eq!(entry(&state, "echo").lifecycle, ActorLifecycle::Dead);
    }

    #[rstest::rstest]
    fn description_is_set_on_creation() {
        // Given an empty dashboard.
        let mut state = DashboardState::new();

        // When marking an actor as starting with a description.
        state.mark_starting("echo", Some("Echoes messages".to_owned()));

        // Then the description is stored.
        assert_eq!(
            entry(&state, "echo").description.as_deref(),
            Some("Echoes messages")
        );
    }

    #[rstest::rstest]
    fn description_is_updated_when_supplied() {
        // Given a dashboard with an actor (no description).
        let mut state = DashboardState::new();
        state.mark_starting("echo", None);

        // When marking running with a description.
        state.mark_running("echo", Some("Echoes messages".to_owned()));

        // Then the description is updated.
        assert_eq!(
            entry(&state, "echo").description.as_deref(),
            Some("Echoes messages")
        );
    }

    #[rstest::rstest]
    fn description_is_preserved_when_not_supplied() {
        // Given a dashboard with an actor that has a description.
        let mut state = DashboardState::new();
        state.mark_starting("echo", Some("Echoes messages".to_owned()));

        // When marking running without supplying a description.
        state.mark_running("echo", None);

        // Then the description is preserved (not overwritten with None).
        assert_eq!(
            entry(&state, "echo").description.as_deref(),
            Some("Echoes messages")
        );
    }

    #[rstest::rstest]
    fn set_status_message_sets_message_on_existing_entry() {
        // Given a dashboard with a running actor.
        let mut state = DashboardState::new();
        state.mark_running("discord", None);

        // When setting a status message.
        state.set_status_message("discord", Some("Connected".to_owned()));

        // Then the status message is set.
        assert_eq!(
            entry(&state, "discord").status_message.as_deref(),
            Some("Connected")
        );
        // And the lifecycle is unchanged.
        assert_eq!(entry(&state, "discord").lifecycle, ActorLifecycle::Running);
    }

    #[rstest::rstest]
    fn set_status_message_creates_entry_if_missing() {
        // Given an empty dashboard.
        let mut state = DashboardState::new();

        // When setting a status message for a new actor.
        state.set_status_message("discord", Some("Connecting".to_owned()));

        // Then the entry is created with the message.
        assert_eq!(
            entry(&state, "discord").status_message.as_deref(),
            Some("Connecting")
        );
        // And defaults to Starting lifecycle.
        assert_eq!(entry(&state, "discord").lifecycle, ActorLifecycle::Starting);
    }

    #[rstest::rstest]
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

    #[rstest::rstest]
    fn select_next_clamps_at_last() {
        // Given 3 actors with selection at index 2.
        let mut state = DashboardState::new();
        state.mark_starting("a", None);
        state.mark_starting("b", None);
        state.mark_starting("c", None);
        state.select_next();
        state.select_next();

        // When selecting next again.
        state.select_next();

        // Then the index stays at 2.
        assert_eq!(state.selected_index(), 2);
    }

    #[rstest::rstest]
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

    #[rstest::rstest]
    fn select_first_goes_to_index_zero() {
        // Given 3 actors with selection at index 2.
        let mut state = DashboardState::new();
        state.mark_starting("a", None);
        state.mark_starting("b", None);
        state.mark_starting("c", None);
        state.select_last();

        // When selecting first.
        state.select_first();

        // Then the selected index is 0.
        assert_eq!(state.selected_index(), 0);
    }

    #[rstest::rstest]
    fn select_last_goes_to_last_index() {
        // Given 3 actors with selection at index 0.
        let mut state = DashboardState::new();
        state.mark_starting("a", None);
        state.mark_starting("b", None);
        state.mark_starting("c", None);

        // When selecting last.
        state.select_last();

        // Then the selected index is 2.
        assert_eq!(state.selected_index(), 2);
    }

    #[rstest::rstest]
    fn select_next_noop_with_no_actors() {
        // Given an empty dashboard.
        let mut state = DashboardState::new();

        // When selecting next.
        state.select_next();

        // Then the index stays at 0.
        assert_eq!(state.selected_index(), 0);
    }

    #[rstest::rstest]
    fn actors_returns_insertion_order() {
        // Given a dashboard populated out of order.
        let mut state = DashboardState::new();
        state.mark_starting("c", None);
        state.mark_starting("a", None);
        state.mark_starting("b", None);

        // When querying actors.
        let names: Vec<&str> = state.actors().iter().map(|e| e.name.as_str()).collect();

        // Then the order matches insertion (c, a, b).
        assert_eq!(names, vec!["c", "a", "b"]);
    }

    #[rstest::rstest]
    fn actors_empty_returns_empty_vec() {
        // Given an empty dashboard.
        let state = DashboardState::new();

        // When querying actors.
        let actors = state.actors();

        // Then the result is empty.
        assert!(actors.is_empty());
    }
}
