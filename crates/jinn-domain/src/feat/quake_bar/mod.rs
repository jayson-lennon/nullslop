//! Quake bar — the global overlay console.
//!
//! A drop-down overlay (a la the Quake/Doom console) pinned to the top of the
//! screen. While the [`FocusScope::QuakeBar`](crate::common::focus::FocusScope)
//! scope is on the stack, it captures every keystroke; only `<esc>` dismisses it.
//!
//! Two writers, two fields on [`QuakeBarState`]:
//! - `input` — the 1-line command input, edited synchronously by the
//!   `IntentHandler` (like every other input popup).
//! - `log`   — the persistent command log, owned solely by the
//!   [`QuakeBarActor`], which is the only writer. Submit routes the typed line
//!   through a [`SubmitQuakeBarCommand`](command::SubmitQuakeBarCommand) so the
//!   actor is the single mutator of the log (future debug commands and event
//!   subscriptions also funnel through the actor).

mod command;
mod intent;
mod quake_bar_actor;
mod render;
pub mod state;

pub(crate) use state::QuakeBarState;
pub(crate) use state::quake_bar_slot;
pub use state::quake_scope;

// Composition seam used by `crate::feat::composition_routes` (the test
// keymap surface). Not part of the slice's public contract.
pub(crate) use intent::attach_quake_bar_rows;
pub(crate) use intent::register_quake_input_hook;

use kameo::actor::Spawn;

/// Activates the quake bar slice: mints the cell, spawns the actor,
/// attaches the route rows, registers the input hook, the overlay
/// geometry, and the render view.
///
/// One call from composition (launch/wiring) is the slice's entire
/// integration surface; commenting it out removes the slice — its keys
/// are never bound and its scope is unreachable — with no other edits.
pub fn activate(services: &mut crate::Services) {
    // Mint the cell: the write handle is shared (by clone) between the
    // actor (log writer) and the intent-handler input hook (input
    // writer); the renderer resolves a read handle.
    let cell = services
        .slices
        .register(quake_bar_slot(), QuakeBarState::default())
        .expect("quake-bar slot is registered exactly once at wiring");

    // Spawn the actor: the log's single writer.
    let deps = crate::common::actor_deps::ActorDeps {
        services: services.clone(),
    };
    let _actor = quake_bar_actor::QuakeBarActor::supervise(
        &services.root_supervisor,
        quake_bar_actor::QuakeBarActorDeps {
            deps,
            cell: cell.clone(),
        },
    )
    .restart_policy(kameo::supervision::RestartPolicy::Never)
    .spawn();

    // Route rows + input hook + overlay geometry + overlay renderer.
    intent::attach_quake_bar_rows(&services.key_routes, &cell);
    intent::register_quake_input_hook(&services.key_routes, &cell);
    let overlay_scope = quake_scope();
    services
        .slices
        .register_overlay(overlay_scope.clone(), std::sync::Arc::new(overlay_rect));
    services
        .slices
        .register_overlay_slot(overlay_scope.clone(), quake_bar_slot());
    services.overlay_views.register(
        overlay_scope,
        std::sync::Arc::new(|frame, area, ctx| render::render_quake_bar(frame, area, ctx)),
    );
}

/// The quake bar overlay's screen rect for a frame of `area`.
///
/// Full-width drop-down pinned just below the tab bar. Registered by
/// `activate`; the generic overlay pass consults it.
fn overlay_rect(area: &ratatui::layout::Rect) -> Option<ratatui::layout::Rect> {
    Some(ratatui::layout::Rect {
        x: area.x,
        y: area.y + 1,
        width: area.width,
        height: area.height.saturating_sub(1),
    })
}
