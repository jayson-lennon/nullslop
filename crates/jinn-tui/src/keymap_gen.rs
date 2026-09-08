//! Slice keybind generation — the bridge between slice route rows and
//! the which-key keymap.
//!
//! Slices declare their keybinds as [`RouteRow`]s (scope + key +
//! outcome) attached to `KeyRoutes` at activation. This module
//! materializes those rows as keymap bindings, once, after all
//! activations: static intents resolve through the `RouteId` → intent
//! table below, dynamic actions bind to `Intent::Dynamic` carrying the
//! row's `(slice, action)` identity. An unregistered slice's keys are
//! simply never bound — removability is automatic, not maintained.

use jinn_domain::Key;
use jinn_domain::KeyEvent;
use jinn_domain::common::slices::key_routes::BindSite;
use jinn_domain::common::slices::key_routes::KeyRoutes;
use jinn_domain::common::slices::key_routes::RouteOutcome;
use jinn_domain::common::slices::key_routes::RouteRow;
use jinn_slices::SliceScopeId;
use ratatui_which_key::Keymap;

use crate::keymap::KeyCategory;
use crate::scope::Scope;
use jinn_domain::Intent;

/// Resolves a static row's [`RouteId`] to the composition intent it
/// binds. Slice keybind blocks used to hardcode these — the table is
/// now the single central record of slice keys that are plain static
/// intents (shared-chrome keys like `q` → quit).
fn static_intent(route_id: &str) -> Option<Intent> {
    match route_id {
        "dashboard:quit" => Some(Intent::Quit),
        "dashboard:switch-tab" => Some(Intent::SwitchTab),
        "dashboard:which-key" => Some(Intent::ToggleWhichkey),
        "quake-bar:ctrl-clear" => Some(Intent::CtrlClear),
        _ => None,
    }
}

/// Maps a row category hint onto the keymap category enum.
fn category(name: &str) -> KeyCategory {
    match name {
        "navigation" => KeyCategory::Navigation,
        "input" => KeyCategory::Input,
        _ => KeyCategory::General,
    }
}

/// The keymap scope a row binds into.
///
/// `OwnScope` rows bind in their slice's dynamic scope;
/// `GlobalToggle` rows bind in every static scope (skipping the
/// slice's own scope, where its own close row wins) and in other
/// slices' dynamic scopes.
fn scopes_for_row<'a>(
    row: &'a RouteRow,
    tabs: &'a [SliceScopeId],
    hooks: &'a [SliceScopeId],
) -> Vec<Scope> {
    let terminal_scopes = [
        Scope::TerminalView,
        Scope::TerminalControl,
    ];
    match row.site {
        BindSite::OwnScope => vec![Scope::Dynamic(row.scope.clone())],
        BindSite::GlobalToggle => {
            let mut scopes: Vec<Scope> = [
                Scope::Normal,
                Scope::Input,
                Scope::ArgInput,
                Scope::TokenBudgetInput,
                Scope::RenameSessionInput,
                Scope::CwdInput,
                Scope::ProjectAddInput,
                Scope::PrunerAccumulationInput,
                Scope::SidebarResize,
                Scope::SidebarPersona,
                Scope::SidebarPins,
                Scope::SidebarSessions,
                Scope::SidebarTaskList,
                Scope::SidebarMcpServers,
                Scope::PickerProvider,
                Scope::PickerSession,
                Scope::PickerPersona,
                Scope::PickerTheme,
                Scope::PickerLifecycle,
                Scope::PickerCompactionModel,
                Scope::PickerReasoningEffort,
                Scope::PickerEndpoint,
                Scope::PickerTool,
                Scope::PickerSkill,
                Scope::PickerTaskList,
                Scope::PickerProject,
                Scope::PickerMcpServer,
                Scope::PickerPlugin,
            ]
            .into_iter()
            .collect();
            for scope in tabs {
                scopes.push(Scope::Dynamic(scope.clone()));
            }
            for scope in hooks {
                if *scope != row.scope {
                    scopes.push(Scope::Dynamic(scope.clone()));
                }
            }
            // Terminal scopes are intentionally excluded: globals would
            // strand the terminal control flag (globals beat catch-alls
            // and pierce capture mode) and pop the passive view.
            debug_assert!(
                !terminal_scopes.contains(&Scope::Dynamic(row.scope.clone())),
                "terminal scopes are static; a slice never binds there"
            );
            scopes
        }
    }
}

/// Collects every dynamic scope the route table knows about: row scopes
/// (tab scopes) plus input-hook scopes (capture scopes). Used to spread
/// per-scope composition chrome (the `<M-t>` toggle) across slices.
#[must_use]
pub fn dynamic_scopes(routes: &KeyRoutes) -> Vec<SliceScopeId> {
    let mut scopes: Vec<SliceScopeId> = routes.rows().iter().map(|r| r.scope.clone()).collect();
    for hook in routes.hook_scopes() {
        if !scopes.contains(&hook) {
            scopes.push(hook);
        }
    }
    scopes
}

/// Materializes every attached route row as keymap bindings.
///
/// Called once after all slice activations, before the event loop. The
/// keymap is mutated in place so the generated bindings land in the
/// same tree as the built-in scope bindings.
pub fn bind_route_rows(
    routes: &KeyRoutes,
    keymap: &mut Keymap<KeyEvent, Scope, Intent, KeyCategory>,
) {
    let rows = routes.rows();
    let hooks = routes.hook_scopes();
    // Row scopes that host other slices' global toggles: every registered
    // scope (rows + hooks) except the row's own, where its OwnScope rows
    // must win.
    let mut tabs: Vec<SliceScopeId> = rows.iter().map(|r| r.scope.clone()).collect();
    for hook in &hooks {
        if !tabs.contains(hook) {
            tabs.push(hook.clone());
        }
    }
    tabs.dedup();
    for row in &rows {
        let category = category(row.category);
        let scopes = scopes_for_row(row, &tabs, &hooks);
        match &row.outcome {
            RouteOutcome::StaticIntent(_) => {
                let Some(intent) = static_intent(row.route_id.as_str()) else {
                    tracing::warn!(
                        route = row.route_id.as_str(),
                        "static route row has no composition intent mapping; key unbound"
                    );
                    continue;
                };
                let display = row.display();
                for scope in scopes {
                    if display.is_empty() {
                        keymap.bind(row.key, intent.clone(), category, scope);
                    } else {
                        keymap.bind(row.key, intent.clone(), category, scope);
                    }
                }
            }
            RouteOutcome::Action {
                action,
                display,
                run: _,
            } => {
                let intent = Intent::Dynamic(jinn_slices::DynamicIntent::new(
                    row.scope.clone(),
                    action,
                    display,
                ));
                for scope in scopes {
                    keymap.bind(row.key, intent.clone(), category, scope);
                }
            }
        }
    }
    // Typing carve-out: a slice with a registered input hook captures
    // printable keystrokes in its own scope. The keymap synthesizes the
    // generic editing intent; the intent handler's hook consult (not a
    // god-match arm) routes it to the slice's sync writer.
    for hook in hooks {
        keymap.scope(Scope::Dynamic(hook), |b| {
            b.catch_all(|key: KeyEvent| {
                if let KeyEvent { key: Key::Char(c), .. } = &key {
                    Some(Intent::InsertChar { ch: *c })
                } else {
                    None
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable,
        clippy::indexing_slicing,
        reason = "test code"
    )]

    use super::bind_route_rows;
    use super::category;
    use super::static_intent;
    use crate::keymap::KeyCategory;
    use crate::scope::Scope;
    use jinn_domain::Intent;
    use jinn_domain::common::slices::key_routes::ActionFn;
use jinn_domain::common::slices::key_routes::BindSite;
    use jinn_domain::common::slices::key_routes::KeyRoutes;
    use jinn_domain::common::slices::key_routes::RouteId;
    use jinn_domain::common::slices::key_routes::RouteOutcome;
    use jinn_domain::common::slices::key_routes::RouteRow;
    use jinn_domain::protocol::IntentResult;
    use jinn_slices::SliceScopeId;
        use ratatui_which_key::Keymap;

    fn quake_open_row() -> RouteRow {
        RouteRow {
            route_id: RouteId::new("quake-bar:open"),
            scope: SliceScopeId::new("quake-bar", "open"),
            key: "<M-`>",
            category: "general",
            site: BindSite::GlobalToggle,
            feature: "quake-bar",
            outcome: RouteOutcome::Action {
                action: "open",
                display: "quake bar",
                run: ActionFn::new(|| IntentResult::empty()),
            },
        }
    }

    fn dashboard_quit_row() -> RouteRow {
        RouteRow {
            route_id: RouteId::new("dashboard:quit"),
            scope: SliceScopeId::new("dashboard", "tab"),
            key: "q",
            category: "general",
            site: BindSite::OwnScope,
            feature: "dashboard",
            outcome: RouteOutcome::StaticIntent(RouteId::new("dashboard:quit")),
        }
    }

    #[rstest::rstest]
    #[test]
    fn dynamic_action_row_binds_dynamic_intent_in_own_scope() {
        // Given a route table with a quake-bar open row (global toggle).
        let routes = KeyRoutes::new();
        routes.attach(quake_open_row());

        // When generating bindings into a fresh keymap.
        let mut keymap = Keymap::new();
        bind_route_rows(&routes, &mut keymap);

        // Then Normal scope carries the toggle as a dynamic intent...
        let bindings = keymap.bindings_for_scope(Scope::Normal);
        assert!(
            bindings
                .iter()
                .any(|group| group
                    .bindings
                    .iter()
                    .any(|b| b.description.contains("quake bar"))),
            "Normal scope should show the open binding"
        );
        // ...and the slice's own scope also carries it: with only this
        // row attached there is no competing close row, so the toggle is
        // the binding the slice's scope resolves (a registered slice's
        // OwnScope close row then shadows it by binding the same key).
    }

    #[rstest::rstest]
    #[test]
    fn global_toggle_row_binds_in_normal_scope() {
        // Given a route table with a quake-bar open row (global toggle).
        let routes = KeyRoutes::new();
        routes.attach(quake_open_row());

        // When generating bindings into a fresh keymap.
        let mut keymap = Keymap::new();
        bind_route_rows(&routes, &mut keymap);

        // Then Normal scope gained the <M-`> binding.
        let bindings = keymap.bindings_for_scope(Scope::Normal);
        assert!(
            bindings
                .iter()
                .any(|group| group
                    .bindings
                    .iter()
                    .any(|b| b.description.contains("quake bar"))),
            "Normal scope should show the quake toggle"
        );
    }

    #[rstest::rstest]
    #[test]
    fn static_row_resolves_through_the_intent_table() {
        // Given a route table with the dashboard quit row.
        let routes = KeyRoutes::new();
        routes.attach(dashboard_quit_row());

        // When resolving the row's route id through the mapping.
        let intent = static_intent("dashboard:quit");

        // Then it resolves to the shared-chrome Quit intent.
        assert_eq!(intent, Some(Intent::Quit));
        // And an unknown route id resolves to nothing (unbound, not guessed).
        assert_eq!(static_intent("dashboard:unknown"), None);
    }

    #[rstest::rstest]
    #[test]
    fn category_hints_map_onto_keymap_categories() {
        // Given the three category hints in use.
        // When mapping them.
        // Then they land on the matching keymap categories.
        assert_eq!(category("navigation"), KeyCategory::Navigation);
        assert_eq!(category("input"), KeyCategory::Input);
        assert_eq!(category("general"), KeyCategory::General);
        // And an unknown hint falls back to General.
        assert_eq!(category("whatever"), KeyCategory::General);
    }
}
