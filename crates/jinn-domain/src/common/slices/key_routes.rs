//! Feature-registered keybind routing.
//!
//! The central intent handler is a god-match: one arm per keybound
//! behavior, hand-written in one place, growing linearly with features.
//! [`KeyRoutes`] dissolves that coupling: each feature registers *route
//! rows* — "when this intent fires, produce this message" — and the
//! handler consults the table by lookup instead of mutating foreign
//! state arm by arm.
//!
//! Rows come in two kinds, mirroring the two kinds of writers:
//!
//! - [`RouteRow::Builtin`] maps an intent to a Rust closure producing
//!   an [`IntentResult`] (typically a single bus message for the
//!   feature's owning actor).
//! - [`RouteRow::Guest`] maps a (scope, key) pair to a coordinator-
//!   addressed action for a WASM plugin. Guests cannot be dispatched to
//!   directly — a slice is storage and a plugin's logic lives behind
//!   its host-side coordinator — so guest rows are data (`plugin`,
//!   `action`) that the runtime resolves against the plugin
//!   coordinator. Data-only in this phase; never dispatched by the
//!   sync path.
//!
//! The table is small and scanned linearly; built-in rows win over
//! guest rows on lookup because they are attached first (startup
//! wiring registers built-ins before plugins load).
//!
//! This module lives in `jinn-domain` (not `jinn-slices`) because its
//! rows are keyed on the central [`Intent`] protocol type — a
//! composition concern. It moves to `jinn-slices` once rows are
//! re-keyed off `Intent`.

use std::mem::Discriminant;

use crate::protocol::intent::Intent;
use crate::protocol::intent::IntentResult;

/// A feature-registered keybind route.
#[derive(Debug, Clone)]
pub enum RouteRow {
    /// A built-in feature's row: intent → message-producing action.
    Builtin {
        /// Which intent this row serves. Stored as a [`DiscriminantKey`]
        /// because `Intent` has data-carrying variants (`InsertChar`
        /// and friends); discriminants key by *shape*, not payload.
        intent: DiscriminantKey,
        /// Produces the messages to publish when the intent fires.
        action: fn() -> IntentResult,
        /// Display name of the owning feature, for diagnostics and the
        /// dashboard's route listing.
        feature: &'static str,
    },
    /// A WASM plugin's row: (scope, key) → coordinator-addressed action.
    ///
    /// Data-only: the plugin coordinator owns dispatch. The sync intent
    /// path never runs guest actions — a guest is remote by
    /// construction, so no sync write handle can reach it.
    Guest {
        /// Keymap scope the guest bound itself to (e.g. `dashboard`).
        scope: String,
        /// The key, in keymap display form (e.g. `ctrl+s`).
        key: String,
        /// The plugin that registered the row.
        plugin: String,
        /// Coordinator-addressed action name.
        action: String,
    },
}

impl RouteRow {
    /// Returns the row's keymap scope, if it declares one.
    #[must_use]
    pub fn scope(&self) -> Option<&str> {
        match self {
            Self::Builtin { .. } => None,
            Self::Guest { scope, .. } => Some(scope),
        }
    }
}

/// Hashable wrapper around [`std::mem::Discriminant<Intent>`].
///
/// `Discriminant` is `Hash` but the bound shows up awkwardly in map
/// keys and diagnostics; this newtype centralizes the indirection and
/// gives the table's key a speakable name. Display strings of intents
/// are deliberately *not* used as keys — they are UI-facing and not
/// stable identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiscriminantKey(Discriminant<Intent>);

impl DiscriminantKey {
    /// Keys by the intent's variant, ignoring payload data.
    #[must_use]
    pub fn of(intent: &Intent) -> Self {
        Self(std::mem::discriminant(intent))
    }
}

/// Registry of feature keybind routes.
///
/// Rows attach after startup wiring (plugins load late), so the table
/// is interior-mutable behind a lock — the same shape as
/// [`Slices`](crate::common::slices::Slices). Lookup is infallible: an
/// unbound intent yields `None` and the handler falls through to its
/// own arms.
#[derive(Clone, Debug, Default)]
pub struct KeyRoutes {
    rows: row_store::Rows,
}

impl KeyRoutes {
    /// Creates an empty route table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Attaches a built-in row: `probe`'s discriminant names the
    /// intent; `action` produces the messages.
    pub fn attach_builtin(
        &self,
        probe: &Intent,
        action: fn() -> IntentResult,
        feature: &'static str,
    ) {
        self.rows.push(RouteRow::Builtin {
            intent: DiscriminantKey::of(probe),
            action,
            feature,
        });
    }

    /// Attaches a guest row for a plugin keybind.
    pub fn attach_guest(&self, scope: &str, key: &str, plugin: &str, action: &str) {
        self.rows.push(RouteRow::Guest {
            scope: scope.to_owned(),
            key: key.to_owned(),
            plugin: plugin.to_owned(),
            action: action.to_owned(),
        });
    }

    /// Looks up the action bound to `intent`, if a built-in row serves it.
    #[must_use]
    pub fn action_for(&self, intent: &Intent) -> Option<IntentResult> {
        let probe = DiscriminantKey::of(intent);
        let action = {
            let rows = self.rows.rows();
            rows.into_iter().find_map(|row| match row {
                RouteRow::Builtin { intent, action, .. } if intent == probe => Some(action),
                _ => None,
            })
        };
        action.map(|action| action())
    }

    /// Looks up a guest row by keymap scope and key.
    ///
    /// Returns an owned row: rows are small data, and a clone spares
    /// callers any lifetime tie to the table's lock.
    #[must_use]
    pub fn guest_row(&self, scope: &str, key: &str) -> Option<RouteRow> {
        let rows = self.rows.rows();
        rows.into_iter().find(|row| match row {
            RouteRow::Guest {
                scope: s, key: k, ..
            } => s == scope && k == key,
            RouteRow::Builtin { .. } => false,
        })
    }

    /// Returns all attached rows in attach order.
    #[must_use]
    pub fn rows(&self) -> Vec<RouteRow> {
        self.rows.rows()
    }
}

/// Append-only row store shared by all clones of the table.
mod row_store {
    use super::RouteRow;
    use parking_lot::RwLock;
    use std::sync::Arc;

    #[derive(Debug, Default)]
    pub struct Rows {
        inner: Arc<RwLock<Vec<RouteRow>>>,
    }

    impl Clone for Rows {
        fn clone(&self) -> Self {
            Self {
                inner: Arc::clone(&self.inner),
            }
        }
    }

    impl Rows {
        pub fn push(&self, row: RouteRow) {
            self.inner.write().push(row);
        }

        /// Snapshot of all rows; the guard is released before return.
        pub fn rows(&self) -> Vec<RouteRow> {
            self.inner.read().clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::KeyRoutes;
    use super::RouteRow;
    use crate::protocol::intent::Intent;
    use crate::protocol::intent::IntentResult;

    #[derive(Debug, Clone)]
    struct StubMsg;

    impl crate::common::bus::BusMessage for StubMsg {}

    fn stub_action() -> IntentResult {
        IntentResult::new_message(StubMsg)
    }

    #[rstest::rstest]
    #[test]
    fn route_lookup_produces_message_for_bound_intent() {
        // Given a table with a row bound to a dashboard nav intent.
        let routes = KeyRoutes::new();
        routes.attach_builtin(&Intent::NoOp, stub_action, "stub");

        // When looking up the action for that intent (data variants
        // must match by discriminant, not payload).
        let result = routes.action_for(&Intent::NoOp);

        // Then the action produced the stub message.
        let result = result.expect("bound intent resolves");
        assert_eq!(result.message_names, vec![std::any::type_name::<StubMsg>()]);
    }

    #[rstest::rstest]
    #[test]
    fn unbound_key_is_noop() {
        // Given a table with no row for quit.
        let routes = KeyRoutes::new();

        // When looking up the action for Intent::Quit.
        let result = routes.action_for(&Intent::Quit);

        // Then nothing resolves and the handler falls through.
        assert!(result.is_none());
    }

    #[rstest::rstest]
    #[test]
    fn guest_route_row_targets_plugin_coordinator() {
        // Given a table with a guest row attached.
        let routes = KeyRoutes::new();
        routes.attach_guest("dashboard", "ctrl+s", "my-plugin", "save");

        // When looking up the guest row by scope and key.
        let row = routes.guest_row("dashboard", "ctrl+s");

        // Then the row carries the plugin and the coordinator-addressed
        // action.
        let row = row.expect("guest row resolves");
        let RouteRow::Guest {
            scope,
            key,
            plugin,
            action,
        } = row
        else {
            unreachable!("attached row should be a guest row");
        };
        assert_eq!(scope, "dashboard");
        assert_eq!(key, "ctrl+s");
        assert_eq!(plugin, "my-plugin");
        assert_eq!(action, "save");
    }
}
