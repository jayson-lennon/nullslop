//! Feature-registered keybind routing — the slice keybind manifest.
//!
//! [`KeyRoutes`] dissolves the central-handler coupling: each slice
//! registers *route rows* — "when this key fires in this scope, produce
//! this outcome" — and composition generates the keymap bindings from
//! the registered rows. Rows carry no `Intent`: a row either resolves
//! through the route table itself (a [`RouteOutcome::Action`], looked up
//! by dynamic intent) or names a static intent by [`RouteId`] for
//! composition to bind directly ([`RouteOutcome::StaticIntent`]). The
//! intent vocabulary therefore lives in exactly one place — the
//! composition-side `RouteId` map — and slices never edit central
//! enums.
//!
//! Rows also declare *where* their key binds: a slice's own dynamic
//! scope ([`BindSite::OwnScope`]) or every composition scope
//! ([`BindSite::GlobalToggle`] — e.g. the key that opens the slice).
//! Composition's generator walks the rows; nothing else does.
//!
//! Alongside the rows, a slice may register one *input hook* per scope
//! ([`InputHook`]): a synchronous interceptor consulted before the
//! handler's built-in arms while that scope is active. This is the
//! sanctioned carve-out for per-keystroke input surfaces — the hook
//! writes the slice's own state synchronously, exactly as a built-in
//! input popup does.
//!
//! The table is small and scanned linearly; rows attach at slice
//! activation (startup wiring) before the keymap is generated.

use std::sync::Arc;

use jinn_slices::SliceScopeId;

use crate::protocol::intent::Intent;
use crate::protocol::intent::IntentResult;

/// Composition-side identifier for a route's intent resolution.
///
/// Rows never name [`Intent`] variants directly — they carry a
/// [`RouteId`], and composition's generator maps ids to intents in one
/// table. A `RouteId` an unknown id to that map is a wiring bug that
/// surfaces as an unbound key at startup, not a compile error; the
/// mapping test pins every registered id against it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RouteId(&'static str);

impl RouteId {
    /// Mints a route id from its canonical dotted name.
    #[must_use]
    pub const fn new(name: &'static str) -> Self {
        Self(name)
    }

    /// The canonical name, e.g. `dashboard:nav-down`.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        self.0
    }
}

/// Where a row's keybinding materializes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindSite {
    /// Bind in the slice's own dynamic scope only.
    OwnScope,
    /// Bind in every composition (static) scope — and in other slices'
    /// dynamic scopes — so a slice's entry-point key works everywhere.
    /// Within the slice's own scope the row is skipped, letting the
    /// slice's own binding (e.g. a close key) win.
    GlobalToggle,
}

/// What a row's keypress produces.
#[derive(Debug, Clone)]
pub enum RouteOutcome {
    /// Bind the key to a static intent, resolved by composition from
    /// the [`RouteId`]. The route table is not consulted at keypress
    /// time — the intent flows through the handler's built-in arms.
    /// Used for a slice's shared-chrome keys (`q` → quit).
    StaticIntent(RouteId),
    /// A slice-specific action: the key binds to a dynamic intent and
    /// the handler dispatches through this row's `run`.
    Action {
        /// The action name — the route-table key within the slice.
        action: &'static str,
        /// Human-readable label for which-key popups.
        display: &'static str,
        /// Produces the outcome when the dynamic intent fires.
        run: ActionFn,
    },
}

/// A row action: produces the intent result (messages + optional scope
/// signal) when its dynamic intent fires.
///
/// A closure, not a bare `fn` pointer: actions may capture the slice's
/// cell handle (e.g. submit reads and clears the input buffer). The
/// captured handle is the one registered at slice activation — closure
/// capture does not mint a second write capability.
#[derive(Clone)]
pub struct ActionFn(Arc<dyn Fn() -> IntentResult + Send + Sync>);

impl ActionFn {
    /// Wraps a closure or function into a row action.
    #[must_use]
    pub fn new<F>(f: F) -> Self
    where
        F: Fn() -> IntentResult + Send + Sync + 'static,
    {
        Self(Arc::new(f))
    }

    /// Runs the action.
    #[must_use]
    pub fn run(&self) -> IntentResult {
        (self.0)()
    }
}

impl std::fmt::Debug for ActionFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ActionFn(..)")
    }
}

/// A slice-registered keybind row — one entry of the slice manifest.
#[derive(Debug, Clone)]
pub struct RouteRow {
    /// Composition-facing id (static resolution + diagnostics).
    pub route_id: RouteId,
    /// The dynamic scope this row's key lives in.
    pub scope: SliceScopeId,
    /// The key, in keymap display form (e.g. `<esc>`, `j`).
    pub key: &'static str,
    /// Keymap category hint: `general`, `navigation`, or `input`.
    pub category: &'static str,
    /// Where the binding materializes.
    pub site: BindSite,
    /// Display name of the owning slice, for diagnostics.
    pub feature: &'static str,
    /// What the keypress produces.
    pub outcome: RouteOutcome,
}

impl RouteRow {
    /// The which-key label this row's key shows.
    ///
    /// Static intents are labeled by composition (the bound intent's
    /// own `Display`); dynamic actions carry their label here.
    #[must_use]
    pub fn display(&self) -> &'static str {
        match &self.outcome {
            RouteOutcome::StaticIntent(_) => "",
            RouteOutcome::Action { display, .. } => display,
        }
    }
}

/// A synchronous per-scope input interceptor.
///
/// Consulted by the intent handler while the hook's scope is the active
/// focus: editing intents (typing, cursor moves) are routed here so the
/// slice's input surface captures keystrokes without hard-coded handler
/// arms. Returning `None` lets the intent fall through to the built-in
/// arms (quit and other app-level intents keep working).
pub type InputHook = Arc<dyn Fn(&Intent) -> Option<IntentResult> + Send + Sync>;

/// Registry of slice keybind routes and input hooks.
///
/// Rows attach at slice activation (startup wiring), so the table is
/// interior-mutable behind a lock — the same shape as
/// [`Slices`](super::Slices). Lookup is infallible: an unbound dynamic
/// intent yields `None` and the handler treats it as inert.
#[derive(Clone, Debug, Default)]
pub struct KeyRoutes {
    rows: row_store::Rows,
    hooks: row_store::Hooks,
}

impl KeyRoutes {
    /// Creates an empty route table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Attaches a built-in row.
    pub fn attach(&self, row: RouteRow) {
        self.rows.push(row);
    }

    /// Registers the synchronous input hook for a slice's scope.
    pub fn register_input_hook(&self, scope: &SliceScopeId, hook: InputHook) {
        self.hooks.insert(scope.key(), hook);
    }

    /// Returns the input hook registered for `scope`, if any.
    #[must_use]
    pub fn input_hook(&self, scope: &SliceScopeId) -> Option<InputHook> {
        self.hooks.get(&scope.key())
    }

    /// Dispatches a dynamic intent through its registered row.
    ///
    /// Matches by `(slice, action)` — the dynamic intent's identity.
    /// `None` means no row serves this intent: the handler treats the
    /// intent as inert.
    #[must_use]
    pub fn action_for(&self, intent: &Intent) -> Option<IntentResult> {
        let jinn_slices::DynamicIntent {
            slice,
            action,
            display: _,
        } = match intent {
            Intent::Dynamic(dynamic) => dynamic,
            _ => return None,
        };
        let run = {
            let rows = self.rows.rows();
            rows.into_iter().find_map(|row| match row.outcome {
                RouteOutcome::Action {
                    action: row_action,
                    display: _,
                    run,
                } if row_action == action && row.scope == *slice => Some(run),
                _ => None,
            })
        };
        Some(run?.run())
    }

    /// Returns all attached rows in attach order.
    #[must_use]
    pub fn rows(&self) -> Vec<RouteRow> {
        self.rows.rows()
    }

    /// Returns the scope ids of all registered input hooks.
    #[must_use]
    pub fn hook_scopes(&self) -> Vec<SliceScopeId> {
        self.hooks
            .keys()
            .into_iter()
            .filter_map(|key| key.parse::<SliceScopeId>().ok())
            .collect()
    }
}

/// Append-only row/hook store shared by all clones of the table.
mod row_store {
    use super::InputHook;
    use super::RouteRow;
    use parking_lot::RwLock;
    use std::collections::HashMap;
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

    #[derive(Debug, Default)]
    pub struct Hooks {
        inner: Arc<RwLock<HashMap<String, HookEntry>>>,
    }

    /// A hook wrapped for `Debug` (closures are not `Debug`).
    #[derive(Clone)]
    struct HookEntry(InputHook);

    impl std::fmt::Debug for HookEntry {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("InputHook(..)")
        }
    }

    impl Clone for Hooks {
        fn clone(&self) -> Self {
            Self {
                inner: Arc::clone(&self.inner),
            }
        }
    }

    impl Hooks {
        pub fn insert(&self, key: String, hook: InputHook) {
            self.inner.write().insert(key, HookEntry(hook));
        }

        pub fn get(&self, key: &str) -> Option<InputHook> {
            self.inner.read().get(key).map(|entry| entry.0.clone())
        }

        pub fn keys(&self) -> Vec<String> {
            self.inner.read().keys().cloned().collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ActionFn;
    use super::BindSite;
    use super::KeyRoutes;
    use super::RouteId;
    use super::RouteOutcome;
    use super::RouteRow;
    use crate::protocol::intent::Intent;
    use crate::protocol::intent::IntentResult;
    use jinn_slices::DynamicIntent;
    use jinn_slices::SliceScopeId;

    fn scope() -> SliceScopeId {
        SliceScopeId::new("test-slice", "main")
    }

    fn row(action: &'static str, key: &'static str) -> RouteRow {
        RouteRow {
            route_id: RouteId::new("test-slice:action"),
            scope: scope(),
            key,
            category: "general",
            site: BindSite::OwnScope,
            feature: "test-slice",
            outcome: RouteOutcome::Action {
                action,
                display: "test action",
                run: ActionFn::new(|| IntentResult::empty()),
            },
        }
    }

    fn dynamic_intent(action: &str) -> Intent {
        Intent::Dynamic(DynamicIntent::new(scope(), action, "test action"))
    }

    #[rstest::rstest]
    #[test]
    fn dynamic_intent_with_registered_row_dispatches_action() {
        // Given a table with an action row attached.
        let routes = KeyRoutes::new();
        routes.attach(row("poke", "<enter>"));

        // When dispatching a dynamic intent carrying the row's action.
        let result = routes.action_for(&dynamic_intent("poke"));

        // Then the row's action ran (empty result, no error).
        assert!(result.is_some());
    }

    #[rstest::rstest]
    #[test]
    fn dynamic_intent_without_row_is_inert() {
        // Given a table with no matching row.
        let routes = KeyRoutes::new();

        // When dispatching an unregistered dynamic intent.
        let result = routes.action_for(&dynamic_intent("missing"));

        // Then nothing resolves — the handler will treat it as inert.
        assert!(result.is_none());
    }

    #[rstest::rstest]
    #[test]
    fn static_intents_never_reach_the_route_table() {
        // Given a table with rows attached.
        let routes = KeyRoutes::new();
        routes.attach(row("poke", "<enter>"));

        // When dispatching a static intent.
        let result = routes.action_for(&Intent::Quit);

        // Then nothing resolves (static intents flow through built-in arms).
        assert!(result.is_none());
    }

    #[rstest::rstest]
    #[test]
    fn input_hook_intercepts_intents_for_its_scope() {
        // Given a table with a hook registered for the scope.
        let routes = KeyRoutes::new();
        routes.register_input_hook(
            &scope(),
            std::sync::Arc::new(|intent: &Intent| {
                if matches!(intent, Intent::DeleteGrapheme) {
                    Some(IntentResult::empty())
                } else {
                    None
                }
            }),
        );

        // When looking up the hook.
        let hook = routes.input_hook(&scope()).expect("hook registered");

        // Then the hook serves the editing intent and declines others.
        assert!(hook(&Intent::DeleteGrapheme).is_some());
        assert!(hook(&Intent::Quit).is_none());
    }

    #[rstest::rstest]
    #[test]
    fn hook_scopes_enumerates_registered_scopes() {
        // Given a table with one hook registered.
        let routes = KeyRoutes::new();
        routes.register_input_hook(&scope(), std::sync::Arc::new(|_: &Intent| None));

        // When enumerating hook scopes.
        let scopes = routes.hook_scopes();

        // Then the registered scope is listed.
        assert_eq!(scopes, vec![scope()]);
    }
}
