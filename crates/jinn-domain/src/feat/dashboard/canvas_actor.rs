//! The dashboard actor — owns the dashboard slice cell on trouper.
//!
//! Aggregates two data sources into a single dashboard view:
//!
//! - **Generic actor lifecycle** — receives the lifecycle events
//!   [`ActorStarting`], [`ActorStarted`], and [`ActorShutdownCompleted`] to
//!   track every actor's `Starting`/`Running`/`Dead` phase.
//! - **Discord connection status** — receives [`DiscordStatusUpdate`]
//!   (republished by [`DiscordStatusActor`] onto the bus, bridged to the
//!   `jinn.fabric` topic), writing the free-form status message into the
//!   discord entry.
//! - **Keyboard navigation** — receives [`DashboardNav`], bridged onto the
//!   `jinn.dashboard` topic from the dashboard feature's keybind rows.
//!
//! This actor owns the dashboard's slice cell exclusively: the cell is
//! minted by [`Slices::register`](crate::common::slices::Slices::register)
//! at actor wiring, and this actor holds the one write handle. The
//! renderer and the intent router resolve read handles. Status sources
//! are symmetric producers: they publish events, and this actor is the
//! single sink.
//!
//! The actor runs on the trouper runtime ([`ServiceActor`] tier: a
//! stateless fold into shared state, no journaling). The kameo→canvas
//! bridge ([`crate::common::canvas_bridge`]) translates the bus messages
//! onto its topics; the cell handle cannot ride the runtime's JSON start
//! args, so it is injected through the builder's
//! [`start_with`](trouper::builder::ServiceBuilder::start_with)
//! override.

use trouper::actor::{MsgHandler, ServiceActor};
use trouper::context::MsgCtx;
use trouper::registry::RegistryError;
use trouper::system::ActorSystem;
use trouper::types::ActorPath;

use crate::common::actor::protocol::event::{ActorShutdownCompleted, ActorStarted, ActorStarting};
use crate::common::canvas_bridge;
use crate::feat::browser_binary_scan::{BinaryFamily, BrowserBinaryVerified};
use crate::feat::dashboard::DashboardState;
use crate::feat::dashboard::nav::DashboardNav;
use crate::feat::discord::DiscordStatusUpdate;
use jinn_slices::TypedCell;

/// Dashboard entry name for the web-fetch actor — the row whose Notes column
/// surfaces the resolved browser backend (Chrome/Chromium/Bundled).
const WEB_FETCH_ENTRY: &str = "web-fetch";

const DISCORD_ACTOR_NAME: &str = "discord";
const DISCORD_DESCRIPTION: &str = "Discord gateway bot [Task]";

/// The dashboard actor on the canvas runtime.
///
/// Receives lifecycle events, [`DiscordStatusUpdate`], and
/// [`DashboardNav`] on its topics, folding all of them into the slice
/// cell.
pub struct DashboardCanvasActor {
    /// The dashboard's slice cell — minted at wiring, owned here.
    cell: TypedCell<DashboardState>,
}

impl ServiceActor for DashboardCanvasActor {
    async fn start(
        _args: &serde_json::Value,
    ) -> Result<Self, trouper::error_stack::Report<RegistryError>> {
        // Never called: the spawn helper injects the cell via `start_with`.
        unreachable!(
            "DashboardCanvasActor is spawned via start_with; start requires the typed cell"
        )
    }
}

impl DashboardCanvasActor {
    /// Spawns the actor at `dashboard` and subscribes it to both its
    /// topics (`jinn.fabric` + `jinn.dashboard`).
    ///
    /// A successful [`ActorSystem::subscribe`] is the ordering guarantee:
    /// the topic cursors are registered, so every later publish reaches
    /// the actor's inbox. This is what lets the activation sequence be
    /// spawn-then-activate-the-world without missed lifecycle events.
    pub fn spawn(
        system: &std::sync::Arc<ActorSystem>,
        cell: TypedCell<DashboardState>,
    ) -> ActorPath {
        let path = trouper::builder::spawn_service_builder::<Self>(system)
            .at(ActorPath::new("dashboard"))
            .start_with({
                let cell = cell.clone();
                move || Box::pin(async move { Ok(Self { cell }) })
            })
            .handles::<ActorStarting>()
            .handles::<ActorStarted>()
            .handles::<ActorShutdownCompleted>()
            .handles::<BrowserBinaryVerified>()
            .handles::<DiscordStatusUpdate>()
            .handles::<DashboardNav>()
            .start();
        system
            .subscribe(&path, &canvas_bridge::fabric_topic(), None)
            .expect("dashboard actor subscribes to the fabric topic");
        system
            .subscribe(&path, &canvas_bridge::dashboard_topic(), None)
            .expect("dashboard actor subscribes to the dashboard topic");
        path
    }

    /// Folds an [`ActorStarting`] into the cell.
    fn apply_starting(&self, msg: &ActorStarting) {
        self.cell
            .update(|s| s.mark_starting(&msg.name, msg.description.clone()));
    }

    /// Folds an [`ActorStarted`] into the cell.
    fn apply_started(&self, msg: &ActorStarted) {
        self.cell
            .update(|s| s.mark_running(&msg.name, msg.description.clone()));
    }

    /// Folds an [`ActorShutdownCompleted`] into the cell.
    fn apply_shutdown(&self, msg: &ActorShutdownCompleted) {
        self.cell.update(|s| s.mark_dead(&msg.name, None));
    }

    /// Folds a [`BrowserBinaryVerified`] into the cell: the web-fetch
    /// entry's Notes column only. Never marks lifecycle — that is the
    /// lifecycle folds' job, and mixing them would race them.
    fn apply_browser(&self, msg: &BrowserBinaryVerified) {
        self.cell
            .update(|s| s.set_status_message(WEB_FETCH_ENTRY, Some(backend_label(msg))));
    }

    /// Folds a [`DiscordStatusUpdate`] into the cell.
    fn apply_discord(&self, msg: &DiscordStatusUpdate) {
        self.cell.update(|s| apply_discord_update(s, msg));
    }

    /// Folds a [`DashboardNav`] into the cell.
    fn apply_nav(&self, msg: &DashboardNav) {
        self.cell.update(|s| match *msg {
            DashboardNav::Up => s.select_prev(),
            DashboardNav::Down => s.select_next(),
            DashboardNav::First => s.select_first(),
            DashboardNav::Last => s.select_last(),
        });
    }
}

impl MsgHandler<ActorStarting> for DashboardCanvasActor {
    async fn handle(&mut self, msg: ActorStarting, _ctx: &mut MsgCtx<'_>) {
        self.apply_starting(&msg);
    }
}

impl MsgHandler<ActorStarted> for DashboardCanvasActor {
    async fn handle(&mut self, msg: ActorStarted, _ctx: &mut MsgCtx<'_>) {
        self.apply_started(&msg);
    }
}

impl MsgHandler<ActorShutdownCompleted> for DashboardCanvasActor {
    async fn handle(&mut self, msg: ActorShutdownCompleted, _ctx: &mut MsgCtx<'_>) {
        self.apply_shutdown(&msg);
    }
}

impl MsgHandler<BrowserBinaryVerified> for DashboardCanvasActor {
    async fn handle(&mut self, msg: BrowserBinaryVerified, _ctx: &mut MsgCtx<'_>) {
        self.apply_browser(&msg);
    }
}

impl MsgHandler<DiscordStatusUpdate> for DashboardCanvasActor {
    async fn handle(&mut self, msg: DiscordStatusUpdate, _ctx: &mut MsgCtx<'_>) {
        self.apply_discord(&msg);
    }
}

impl MsgHandler<DashboardNav> for DashboardCanvasActor {
    async fn handle(&mut self, msg: DashboardNav, _ctx: &mut MsgCtx<'_>) {
        self.apply_nav(&msg);
    }
}

/// Builds the dashboard Notes string for a resolved browser binary.
///
/// Format: `"<family> <version>"` (or the bundled/undetected variants),
/// optionally suffixed with `" — <path>"` when a path is known, and
/// optionally prefixed with `"<note>: "` when resolution fell back.
fn backend_label(msg: &BrowserBinaryVerified) -> String {
    let label = match msg.family {
        BinaryFamily::Chrome | BinaryFamily::Chromium => {
            let family = family_display(msg.family);
            match &msg.version_major {
                Some(v) => format!("{family} {v}"),
                None => format!(
                    "{family} {} (version undetected)",
                    jinn_web_fetch::stealth::CHROME_MAJOR
                ),
            }
        }
        BinaryFamily::Bundled => "Chromium (bundled, version undetected)".to_owned(),
    };

    let with_path = match &msg.path {
        Some(p) => format!("{label} — {}", p.display()),
        None => label,
    };

    match &msg.fallback_note {
        Some(note) => format!("{note}: {with_path}"),
        None => with_path,
    }
}

/// Returns the capitalized family name for display.
fn family_display(family: BinaryFamily) -> &'static str {
    match family {
        BinaryFamily::Chrome => "Chrome",
        BinaryFamily::Chromium => "Chromium",
        BinaryFamily::Bundled => "Bundled",
    }
}

/// Apply a discord connection status update to the dashboard state.
fn apply_discord_update(dashboard: &mut DashboardState, update: &DiscordStatusUpdate) {
    let message = update.full_message();
    let (lifecycle, with_description) = match update {
        DiscordStatusUpdate::Connecting => {
            // Ensure the discord entry exists with a description even
            // before Connected/Error arrives. The gateway task is not
            // an actor, so it doesn't emit ActorStarting.
            (Some(crate::feat::dashboard::ActorLifecycle::Starting), true)
        }
        // Disconnected only updates the status message — the lifecycle
        // (Starting/Running/Dead) is driven by the other update variants.
        DiscordStatusUpdate::Disconnected => (None, false),
        DiscordStatusUpdate::Connected => {
            // The gateway task is not an actor, so it doesn't emit
            // ActorStarted. Mark it running here.
            (Some(crate::feat::dashboard::ActorLifecycle::Running), true)
        }
        DiscordStatusUpdate::Error { .. } => {
            // The description is a constant for the discord entry;
            // attach it on creation even when Error arrives first
            // (e.g. missing token).
            (Some(crate::feat::dashboard::ActorLifecycle::Dead), true)
        }
    };

    if let Some(lifecycle) = lifecycle {
        let description = with_description.then(|| DISCORD_DESCRIPTION.to_owned());
        match lifecycle {
            crate::feat::dashboard::ActorLifecycle::Starting => {
                dashboard.mark_starting(DISCORD_ACTOR_NAME, description);
            }
            crate::feat::dashboard::ActorLifecycle::Running => {
                dashboard.mark_running(DISCORD_ACTOR_NAME, description);
            }
            crate::feat::dashboard::ActorLifecycle::Dead => {
                dashboard.mark_dead(DISCORD_ACTOR_NAME, description);
            }
        }
    }
    dashboard.set_status_message(DISCORD_ACTOR_NAME, Some(message));
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
    use super::*;
    use crate::common::slices::Slices;
    use crate::feat::dashboard::ActorLifecycle;
    use crate::feat::dashboard::dashboard_slot;

    /// Polls `check` until it passes or the bounded retry budget runs out.
    async fn wait_for(check: impl Fn() -> bool) {
        for _ in 0..200 {
            if check() {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        panic!("condition never held within the retry budget");
    }

    fn dashboard_entry(
        cell: &TypedCell<DashboardState>,
        name: &str,
    ) -> Option<(ActorLifecycle, Option<String>, Option<String>)> {
        let s = cell.read();
        s.actors()
            .iter()
            .find(|e| e.name == name)
            .map(|e| (e.lifecycle, e.status_message.clone(), e.description.clone()))
    }

    /// Wires one slice cell + dashboard canvas actor into an existing
    /// services container (bridge assumed spawned by the caller).
    fn wire_actor(services: &crate::Services) -> TypedCell<DashboardState> {
        let slices = Slices::new();
        let cell = slices
            .register(dashboard_slot(), DashboardState::new())
            .expect("fresh registry");
        DashboardCanvasActor::spawn(&services.trouper_system, cell.clone());
        cell
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn actor_starting_event_creates_entry_with_starting_lifecycle() {
        // Given a dashboard canvas actor wired behind the bridge.
        let services = crate::Services::new_fake().await;
        canvas_bridge::spawn(&services).await;
        let cell = wire_actor(&services);

        // When publishing ActorStarting on the kameo bus.
        services
            .bus
            .publish(ActorStarting {
                name: "llm".to_owned(),
                description: None,
            })
            .await;

        // Then the dashboard shows the actor as Starting.
        wait_for(|| {
            dashboard_entry(&cell, "llm").is_some_and(|(l, _, _)| l == ActorLifecycle::Starting)
        })
        .await;
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn actor_started_event_transitions_to_running() {
        // Given a dashboard canvas actor wired behind the bridge.
        let services = crate::Services::new_fake().await;
        canvas_bridge::spawn(&services).await;
        let slices = Slices::new();
        let cell = slices
            .register(dashboard_slot(), DashboardState::new())
            .expect("fresh registry");
        DashboardCanvasActor::spawn(&services.trouper_system, cell.clone());

        // When publishing ActorStarted on the kameo bus.
        services
            .bus
            .publish(ActorStarted {
                name: "llm".to_owned(),
                description: None,
            })
            .await;

        // Then the dashboard shows the actor as Running.
        wait_for(|| {
            dashboard_entry(&cell, "llm").is_some_and(|(l, _, _)| l == ActorLifecycle::Running)
        })
        .await;
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn actor_shutdown_event_transitions_to_dead() {
        // Given a dashboard canvas actor whose llm entry is already Running.
        let services = crate::Services::new_fake().await;
        canvas_bridge::spawn(&services).await;
        let slices = Slices::new();
        let cell = slices
            .register(dashboard_slot(), DashboardState::new())
            .expect("fresh registry");
        DashboardCanvasActor::spawn(&services.trouper_system, cell.clone());
        services
            .bus
            .publish(ActorStarted {
                name: "llm".to_owned(),
                description: None,
            })
            .await;
        wait_for(|| {
            dashboard_entry(&cell, "llm").is_some_and(|(l, _, _)| l == ActorLifecycle::Running)
        })
        .await;

        // When publishing ActorShutdownCompleted on the kameo bus.
        services
            .bus
            .publish(ActorShutdownCompleted {
                name: "llm".to_owned(),
            })
            .await;

        // Then the dashboard shows the actor as Dead.
        wait_for(|| {
            dashboard_entry(&cell, "llm").is_some_and(|(l, _, _)| l == ActorLifecycle::Dead)
        })
        .await;
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn discord_connecting_update_sets_status_message_via_bus() {
        // Given a dashboard canvas actor wired behind the bridge.
        let services = crate::Services::new_fake().await;
        canvas_bridge::spawn(&services).await;
        let slices = Slices::new();
        let cell = slices
            .register(dashboard_slot(), DashboardState::new())
            .expect("fresh registry");
        DashboardCanvasActor::spawn(&services.trouper_system, cell.clone());

        // When publishing a Connecting update on the bus (as DiscordStatusActor does).
        services.bus.publish(DiscordStatusUpdate::Connecting).await;

        // Then the dashboard shows the status message.
        wait_for(|| {
            dashboard_entry(&cell, "discord")
                .is_some_and(|(_, m, _)| m.as_deref() == Some("Connecting"))
        })
        .await;
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn discord_connected_update_marks_running_with_message_via_bus() {
        // Given a dashboard canvas actor wired behind the bridge.
        let services = crate::Services::new_fake().await;
        canvas_bridge::spawn(&services).await;
        let slices = Slices::new();
        let cell = slices
            .register(dashboard_slot(), DashboardState::new())
            .expect("fresh registry");
        DashboardCanvasActor::spawn(&services.trouper_system, cell.clone());

        // When publishing a Connected update on the bus.
        services.bus.publish(DiscordStatusUpdate::Connected).await;

        // Then the dashboard shows Running + Connected.
        wait_for(|| {
            dashboard_entry(&cell, "discord").is_some_and(|(l, m, _)| {
                l == ActorLifecycle::Running && m.as_deref() == Some("Connected")
            })
        })
        .await;
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn discord_error_update_marks_dead_with_error_message_via_bus() {
        // Given a dashboard canvas actor wired behind the bridge.
        let services = crate::Services::new_fake().await;
        canvas_bridge::spawn(&services).await;
        let slices = Slices::new();
        let cell = slices
            .register(dashboard_slot(), DashboardState::new())
            .expect("fresh registry");
        DashboardCanvasActor::spawn(&services.trouper_system, cell.clone());

        // When publishing an Error update on the bus.
        services
            .bus
            .publish(DiscordStatusUpdate::Error {
                message: "401: invalid token".to_owned(),
            })
            .await;

        // Then the dashboard shows Dead + the error message.
        wait_for(|| {
            dashboard_entry(&cell, "discord").is_some_and(|(l, m, _)| {
                l == ActorLifecycle::Dead && m.as_deref() == Some("Error: 401: invalid token")
            })
        })
        .await;
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn discord_error_update_first_still_sets_description() {
        // Given a dashboard canvas actor (simulating missing-token: Error arrives first).
        let services = crate::Services::new_fake().await;
        canvas_bridge::spawn(&services).await;
        let slices = Slices::new();
        let cell = slices
            .register(dashboard_slot(), DashboardState::new())
            .expect("fresh registry");
        DashboardCanvasActor::spawn(&services.trouper_system, cell.clone());

        // When publishing an Error update as the very first message.
        services
            .bus
            .publish(DiscordStatusUpdate::Error {
                message: "no token configured".to_owned(),
            })
            .await;

        // Then the entry is created with the discord description.
        wait_for(|| {
            dashboard_entry(&cell, "discord")
                .is_some_and(|(_, _, d)| d.as_deref() == Some("Discord gateway bot [Task]"))
        })
        .await;
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn browser_binary_verified_writes_chrome_label_to_web_fetch_notes() {
        // Given a dashboard canvas actor wired behind the bridge.
        let services = crate::Services::new_fake().await;
        canvas_bridge::spawn(&services).await;
        let slices = Slices::new();
        let cell = slices
            .register(dashboard_slot(), DashboardState::new())
            .expect("fresh registry");
        DashboardCanvasActor::spawn(&services.trouper_system, cell.clone());

        // When publishing BrowserBinaryVerified for a system Chrome.
        services
            .bus
            .publish(BrowserBinaryVerified {
                family: BinaryFamily::Chrome,
                path: Some(std::path::PathBuf::from("/usr/bin/google-chrome")),
                version_major: Some("138".to_owned()),
                fallback_note: None,
            })
            .await;

        // Then the web-fetch row's Notes column carries the backend label.
        wait_for(|| {
            dashboard_entry(&cell, "web-fetch").is_some_and(|(_, m, _)| {
                m.as_deref() == Some("Chrome 138 — /usr/bin/google-chrome")
            })
        })
        .await;
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn browser_binary_verified_writes_bundled_label_to_web_fetch_notes() {
        // Given a dashboard canvas actor wired behind the bridge.
        let services = crate::Services::new_fake().await;
        canvas_bridge::spawn(&services).await;
        let slices = Slices::new();
        let cell = slices
            .register(dashboard_slot(), DashboardState::new())
            .expect("fresh registry");
        DashboardCanvasActor::spawn(&services.trouper_system, cell.clone());

        // When publishing BrowserBinaryVerified for the bundled binary.
        services
            .bus
            .publish(BrowserBinaryVerified {
                family: BinaryFamily::Bundled,
                path: None,
                version_major: None,
                fallback_note: Some("No system Chrome/Chromium — using bundled".to_owned()),
            })
            .await;

        // Then the web-fetch row's Notes column shows the bundled label with note.
        wait_for(|| {
            dashboard_entry(&cell, "web-fetch").is_some_and(|(_, m, _)| {
                m.as_deref()
                    == Some(
                        "No system Chrome/Chromium — using bundled: Chromium (bundled, version undetected)",
                    )
            })
        })
        .await;
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn browser_binary_verified_shows_fallback_version_when_undetected() {
        // Given a dashboard canvas actor wired behind the bridge.
        let services = crate::Services::new_fake().await;
        canvas_bridge::spawn(&services).await;
        let slices = Slices::new();
        let cell = slices
            .register(dashboard_slot(), DashboardState::new())
            .expect("fresh registry");
        DashboardCanvasActor::spawn(&services.trouper_system, cell.clone());

        // When publishing BrowserBinaryVerified for a system Chromium with no version.
        services
            .bus
            .publish(BrowserBinaryVerified {
                family: BinaryFamily::Chromium,
                path: Some(std::path::PathBuf::from("/usr/bin/chromium")),
                version_major: None,
                fallback_note: None,
            })
            .await;

        // Then the displayed version falls back to CHROME_MAJOR so it matches the UA.
        let expected = format!(
            "Chromium {} (version undetected) — /usr/bin/chromium",
            jinn_web_fetch::stealth::CHROME_MAJOR
        );
        wait_for(|| {
            dashboard_entry(&cell, "web-fetch")
                .is_some_and(|(_, m, _)| m.as_deref() == Some(expected.as_str()))
        })
        .await;
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn browser_binary_verified_does_not_create_phantom_entry() {
        // Given a dashboard canvas actor wired behind the bridge.
        let services = crate::Services::new_fake().await;
        canvas_bridge::spawn(&services).await;
        let slices = Slices::new();
        let cell = slices
            .register(dashboard_slot(), DashboardState::new())
            .expect("fresh registry");
        DashboardCanvasActor::spawn(&services.trouper_system, cell.clone());

        // When publishing BrowserBinaryVerified.
        services
            .bus
            .publish(BrowserBinaryVerified {
                family: BinaryFamily::Bundled,
                path: None,
                version_major: None,
                fallback_note: None,
            })
            .await;
        // And giving the pipeline a moment to deliver anything it would.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Then no phantom web-fetch-browser entry is created.
        assert!(dashboard_entry(&cell, "web-fetch-browser").is_none());
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn dashboard_nav_command_moves_selection() {
        // Given a dashboard canvas actor with three actor rows.
        let services = crate::Services::new_fake().await;
        canvas_bridge::spawn(&services).await;
        let slices = Slices::new();
        let cell = slices
            .register(dashboard_slot(), DashboardState::new())
            .expect("fresh registry");
        DashboardCanvasActor::spawn(&services.trouper_system, cell.clone());
        for name in ["a", "b", "c"] {
            services
                .bus
                .publish(ActorStarted {
                    name: name.to_owned(),
                    description: None,
                })
                .await;
        }
        wait_for(|| dashboard_entry(&cell, "c").is_some()).await;

        // When a DashboardNav::Down message arrives.
        services.bus.publish(DashboardNav::Down).await;

        // Then the selection moved to index 1.
        wait_for(|| cell.read().selected_index() == 1).await;
    }

    /// Publishes an `ActorStarted` only AFTER the actor wiring has fully
    /// returned, proving the topic cursor was registered during wiring —
    /// the no-missed-lifecycle-events property the activation sequence
    /// depends on.
    #[rstest::rstest]
    #[tokio::test]
    async fn events_published_after_wiring_are_not_missed() {
        // Given a canvas system with the bridge already spawned.
        let services = crate::Services::new_fake().await;
        canvas_bridge::spawn(&services).await;

        // When the dashboard activates (spawn + subscribe) and only then
        // an ActorStarted is published.
        let slices = Slices::new();
        let cell = slices
            .register(dashboard_slot(), DashboardState::new())
            .expect("fresh registry");
        DashboardCanvasActor::spawn(&services.trouper_system, cell.clone());
        services
            .bus
            .publish(ActorStarted {
                name: "late".to_owned(),
                description: None,
            })
            .await;

        // Then the entry still lands — nothing was missed.
        wait_for(|| dashboard_entry(&cell, "late").is_some()).await;
    }
}
