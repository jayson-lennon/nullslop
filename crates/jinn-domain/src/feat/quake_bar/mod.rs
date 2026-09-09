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
//!   [`QuakeBarCanvasActor`](canvas_actor::QuakeBarCanvasActor), which is the
//!   only writer. Submit routes the typed line through a
//!   [`SubmitQuakeBarCommand`](command::SubmitQuakeBarCommand) so the actor is
//!   the single mutator of the log (future debug commands and event
//!   subscriptions also funnel through the actor).
//!
//! The log writer runs on the trouper runtime (see
//! [`canvas_actor`]); the kameo→trouper bridge translates the bus
//! command onto the `jinn.quake-bar` topic it subscribes to.

pub mod canvas_actor;
pub(crate) mod command;
pub(crate) mod intent;
mod render;
pub mod state;

pub(crate) use state::QuakeBarState;
pub(crate) use state::quake_bar_slot;
pub use state::quake_scope;

// Composition seam used by `crate::feat::composition_routes` (the test
// keymap surface). Not part of the slice's public contract.
pub(crate) use intent::attach_quake_bar_rows;
pub(crate) use intent::register_quake_input_hook;

/// Activates the quake bar slice: mints the cell, spawns the canvas
/// actor, attaches the route rows, registers the input hook, the overlay
/// geometry, and the render view.
///
/// One call from composition (launch/wiring) is the slice's entire
/// integration surface; commenting it out removes the slice — its keys
/// are never bound and its scope is unreachable — with no other edits.
///
/// # Panics
///
/// Panics if the quake-bar cell is already registered — activate must run
/// exactly once (double activation is a wiring bug).
pub fn activate(services: &mut crate::Services) {
    // Mint the cell: the write handle is shared (by clone) between the
    // canvas actor (log writer) and the intent-handler input hook (input
    // writer); the renderer resolves a read handle.
    #[expect(
        clippy::expect_used,
        reason = "double activation is a wiring bug; the slice must activate exactly once"
    )]
    let cell = services
        .slices
        .register(quake_bar_slot(), QuakeBarState::default())
        .expect("quake-bar slot is registered exactly once at wiring");

    // Spawn the canvas actor: the log's single writer. Subscribe returns
    // only after the topic cursor is registered, so no later publish is
    // missed. The bridge (spawned earlier in wiring) feeds the topic.
    canvas_actor::QuakeBarCanvasActor::spawn(&services.trouper_system, &cell);

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
    services
        .overlay_views
        .register(overlay_scope, std::sync::Arc::new(render::render_quake_bar));
}

/// The quake bar overlay's screen rect for a frame of `area`.
///
/// Full-width drop-down pinned just below the tab bar. Registered by
/// `activate`; the generic overlay pass consults it.
// The signature is the `OverlayFn` contract (`Fn(&Rect) -> Option<Rect>`),
// not a free choice — clippy's wraps/borrow lints are expected here.
#[expect(
    clippy::unnecessary_wraps,
    clippy::trivially_copy_pass_by_ref,
    reason = "OverlayFn contract"
)]
fn overlay_rect(area: &ratatui::layout::Rect) -> Option<ratatui::layout::Rect> {
    Some(ratatui::layout::Rect {
        x: area.x,
        y: area.y + 1,
        width: area.width,
        height: area.height.saturating_sub(1),
    })
}
