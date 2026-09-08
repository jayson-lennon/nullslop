//! Feature modules - domain-specific logic, actors, and UI elements.

pub mod auto_prune_worker;
pub mod browser;
pub mod browser_binary_scan;
pub mod chat_entry_selection;
pub mod chat_input;
pub mod compaction_worker;
pub mod context;
pub mod cwd_input;
pub mod dashboard;
pub mod discord;
pub mod discovery;
pub mod discovery_coordinator;
pub mod discovery_notifier;
pub mod endpoint;
pub mod file_lister;
pub mod global;
pub mod history_worker;
pub mod image_convert;
pub mod install;
pub mod intent;
pub mod interactive_term;
pub mod llm_actor;
pub mod mcp;
pub mod mcp_actor;
pub mod mcp_coordinator_actor;
pub mod navigation;
pub mod persona;
pub mod picker;
pub mod plugin;
pub mod plugin_actor;
pub mod plugin_coordinator_actor;
pub mod preferences_actor;
pub mod project;
pub mod project_add_input;
pub mod provider;
pub use jinn_provider_config as provider_infra;
pub mod pruner_accumulation_input;
pub mod quake_bar;
pub mod queue_actor;
pub mod reasoning;
pub mod rename_session_input;
pub mod session;
pub mod session_lifecycle;
pub mod sidebar_resize;
pub mod skills;
pub mod theme;
pub mod todo_list;
pub mod token_count_actor;
pub mod tools_actor;
pub mod ui;
pub mod web_fetch_actor;
pub mod web_search_actor;

/// A `KeyRoutes` pre-seeded with every built-in slice's rows, mirroring
/// what composition produces at launch (all `activate()` calls made).
///
/// Test-only seam: keymap tests query [`crate::feat`] consumers like the
/// quake toggle without standing up the actor system.
#[must_use]
pub fn composition_routes() -> crate::common::slices::key_routes::KeyRoutes {
    let routes = crate::common::slices::key_routes::KeyRoutes::new();
    dashboard::attach_dashboard_rows(&routes);
    // The quake rows' submit/scroll actions capture a cell handle; the
    // seam mints a detached one (never registered into a live `Slices`)
    // since only row *shape* matters for keymap tests.
    let slices = crate::common::slices::Slices::new();
    let cell = slices
        .register(quake_bar::quake_bar_slot(), quake_bar::QuakeBarState::default())
        .expect("detached quake cell");
    quake_bar::attach_quake_bar_rows(&routes, &cell);
    quake_bar::register_quake_input_hook(&routes, &cell);
    routes
}
