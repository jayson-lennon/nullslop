//! The dashboard actor — owns the dashboard slice cell.
//!
//! Aggregates two data sources into a single dashboard view:
//!
//! - **Generic actor lifecycle** — subscribes to the existing bus events
//!   [`ActorStarting`], [`ActorStarted`], and [`ActorShutdownCompleted`] to
//!   track every actor's `Starting`/`Running`/`Dead` phase.
//! - **Discord connection status** — subscribes to [`DiscordStatusUpdate`]
//!   (republished by [`DiscordStatusActor`] from the gateway kanal channel),
//!   writing the free-form status message into the discord entry.
//! - **Keyboard navigation** — subscribes to [`DashboardNav`], routed from
//!   the dashboard feature's keybind rows.
//!
//! This actor owns the dashboard's slice cell exclusively: the cell is
//! minted by [`Slices::register`](crate::common::slices::Slices::register)
//! at actor wiring, and this actor holds the one write handle. The
//! renderer and the intent router resolve read handles. Status sources
//! are symmetric producers: they publish events, and this actor is the
//! single sink.

use kameo::actor::ActorRef;
use kameo::prelude::{Actor, Context, Message};

use crate::common::actor::protocol::event::{ActorShutdownCompleted, ActorStarted, ActorStarting};
use crate::common::actor_deps::ActorDeps;
use crate::common::slices::TypedCell;
use crate::feat::browser_binary_scan::{BinaryFamily, BrowserBinaryVerified};
use crate::feat::dashboard::nav::DashboardNav;
use crate::feat::dashboard::{DashboardState, status_actor::DiscordStatusUpdate};

/// Dashboard entry name for the web-fetch actor — the row whose Notes column
/// surfaces the resolved browser backend (Chrome/Chromium/Bundled).
const WEB_FETCH_ENTRY: &str = "web-fetch";

const DISCORD_ACTOR_NAME: &str = "discord";
const DISCORD_DESCRIPTION: &str = "Discord gateway bot [Task]";

/// The dashboard actor.
///
/// Subscribes to generic lifecycle events, [`DiscordStatusUpdate`], and
/// [`DashboardNav`], and folds all of them into the slice cell.
pub struct DashboardActor {
    cell: TypedCell<DashboardState>,
}

/// Dependencies for [`DashboardActor`].
#[derive(Clone)]
pub struct DashboardActorDeps {
    /// Universal actor dependencies (bus subscription handle).
    pub deps: ActorDeps,
    /// The dashboard's slice cell — minted at wiring, owned here.
    pub cell: TypedCell<DashboardState>,
}

impl Actor for DashboardActor {
    type Args = DashboardActorDeps;
    type Error = kameo::error::Infallible;

    async fn on_start(args: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        args.deps
            .subscribe(actor_ref.clone().recipient::<ActorStarting>())
            .await;
        args.deps
            .subscribe(actor_ref.clone().recipient::<ActorStarted>())
            .await;
        args.deps
            .subscribe(actor_ref.clone().recipient::<ActorShutdownCompleted>())
            .await;
        args.deps
            .subscribe(actor_ref.clone().recipient::<BrowserBinaryVerified>())
            .await;
        args.deps
            .subscribe(actor_ref.clone().recipient::<DiscordStatusUpdate>())
            .await;
        args.deps
            .subscribe(actor_ref.recipient::<DashboardNav>())
            .await;

        Ok(Self { cell: args.cell })
    }
}

impl Message<ActorStarting> for DashboardActor {
    type Reply = ();

    async fn handle(&mut self, msg: ActorStarting, _ctx: &mut Context<Self, Self::Reply>) {
        self.cell.update(|s| {
            s.mark_starting(&msg.name, msg.description);
        });
    }
}

impl Message<ActorStarted> for DashboardActor {
    type Reply = ();

    async fn handle(&mut self, msg: ActorStarted, _ctx: &mut Context<Self, Self::Reply>) {
        self.cell.update(|s| {
            s.mark_running(&msg.name, msg.description);
        });
    }
}

impl Message<ActorShutdownCompleted> for DashboardActor {
    type Reply = ();

    async fn handle(&mut self, msg: ActorShutdownCompleted, _ctx: &mut Context<Self, Self::Reply>) {
        self.cell.update(|s| {
            s.mark_dead(&msg.name, None);
        });
    }
}

impl Message<BrowserBinaryVerified> for DashboardActor {
    type Reply = ();

    async fn handle(&mut self, msg: BrowserBinaryVerified, _ctx: &mut Context<Self, Self::Reply>) {
        self.cell.update(|s| {
            // The web-fetch entry's lifecycle is owned by the actor-lifecycle
            // subscription; we only write the Notes column here. Never call
            // mark_running/mark_dead — that would race the lifecycle handler.
            s.set_status_message(WEB_FETCH_ENTRY, Some(backend_label(&msg)));
        });
    }
}

impl Message<DiscordStatusUpdate> for DashboardActor {
    type Reply = ();

    async fn handle(&mut self, msg: DiscordStatusUpdate, _ctx: &mut Context<Self, Self::Reply>) {
        self.cell.update(|s| apply_discord_update(s, &msg));
    }
}

impl Message<DashboardNav> for DashboardActor {
    type Reply = ();

    async fn handle(&mut self, msg: DashboardNav, _ctx: &mut Context<Self, Self::Reply>) {
        self.cell.update(|s| match msg {
            DashboardNav::Up => s.select_prev(),
            DashboardNav::Down => s.select_next(),
            DashboardNav::First => s.select_first(),
            DashboardNav::Last => s.select_last(),
        });
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
            // a kameo actor, so it doesn't emit ActorStarting.
            (Some(crate::feat::dashboard::ActorLifecycle::Starting), true)
        }
        // Disconnected only updates the status message — the lifecycle
        // (Starting/Running/Dead) is driven by the other update variants.
        DiscordStatusUpdate::Disconnected => (None, false),
        DiscordStatusUpdate::Connected => {
            // The gateway task is not a kameo actor, so it doesn't emit
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
    use crate::common::bus::test_harness::TestHarness;
    use crate::common::slices::Slices;
    use crate::feat::dashboard::ActorLifecycle;
    use crate::feat::dashboard::dashboard_slot;
    use kameo::actor::Spawn;

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

    async fn spawn_actor(
        harness: &TestHarness,
    ) -> (ActorRef<DashboardActor>, TypedCell<DashboardState>) {
        let slices = Slices::new();
        let cell = slices
            .register(dashboard_slot(), DashboardState::new())
            .expect("fresh registry");
        let actor = DashboardActor::spawn(DashboardActorDeps {
            deps: harness.actor_deps().await,
            cell: cell.clone(),
        });
        actor.wait_for_startup().await;
        (actor, cell)
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn actor_starting_event_creates_entry_with_starting_lifecycle() {
        // Given a DashboardActor.
        let harness = TestHarness::new().await;
        let (_actor, cell) = spawn_actor(&harness).await;

        // When publishing ActorStarting.
        harness
            .publish(ActorStarting {
                name: "llm".to_owned(),
                description: None,
            })
            .await;

        // Then the dashboard shows the actor as Starting.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let (lifecycle, _, _) = dashboard_entry(&cell, "llm").expect("entry should exist");
        assert_eq!(lifecycle, ActorLifecycle::Starting);
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn actor_started_event_transitions_to_running() {
        // Given a DashboardActor.
        let harness = TestHarness::new().await;
        let (_actor, cell) = spawn_actor(&harness).await;

        // When publishing ActorStarted.
        harness
            .publish(ActorStarted {
                name: "llm".to_owned(),
                description: None,
            })
            .await;

        // Then the dashboard shows the actor as Running.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let (lifecycle, _, _) = dashboard_entry(&cell, "llm").expect("entry should exist");
        assert_eq!(lifecycle, ActorLifecycle::Running);
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn actor_shutdown_event_transitions_to_dead() {
        // Given a DashboardActor with a running actor entry.
        let harness = TestHarness::new().await;
        let (_actor, cell) = spawn_actor(&harness).await;
        harness
            .publish(ActorStarted {
                name: "llm".to_owned(),
                description: None,
            })
            .await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // When publishing ActorShutdownCompleted.
        harness
            .publish(ActorShutdownCompleted {
                name: "llm".to_owned(),
            })
            .await;

        // Then the dashboard shows the actor as Dead.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let (lifecycle, _, _) = dashboard_entry(&cell, "llm").expect("entry should exist");
        assert_eq!(lifecycle, ActorLifecycle::Dead);
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn discord_connecting_update_sets_status_message_via_bus() {
        // Given a DashboardActor subscribed to DiscordStatusUpdate.
        let harness = TestHarness::new().await;
        let (_actor, cell) = spawn_actor(&harness).await;

        // When publishing a Connecting update on the bus (as DiscordStatusActor now does).
        harness.publish(DiscordStatusUpdate::Connecting).await;

        // Then the dashboard shows the status message.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let (_, message, _) = dashboard_entry(&cell, "discord").expect("entry should exist");
        assert_eq!(message.as_deref(), Some("Connecting"));
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn discord_connected_update_marks_running_with_message_via_bus() {
        // Given a DashboardActor subscribed to DiscordStatusUpdate.
        let harness = TestHarness::new().await;
        let (_actor, cell) = spawn_actor(&harness).await;

        // When publishing a Connected update on the bus.
        harness.publish(DiscordStatusUpdate::Connected).await;

        // Then the dashboard shows Running + Connected.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let (lifecycle, message, _) =
            dashboard_entry(&cell, "discord").expect("entry should exist");
        assert_eq!(lifecycle, ActorLifecycle::Running);
        assert_eq!(message.as_deref(), Some("Connected"));
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn discord_error_update_marks_dead_with_error_message_via_bus() {
        // Given a DashboardActor subscribed to DiscordStatusUpdate.
        let harness = TestHarness::new().await;
        let (_actor, cell) = spawn_actor(&harness).await;

        // When publishing an Error update on the bus.
        harness
            .publish(DiscordStatusUpdate::Error {
                message: "401: invalid token".to_owned(),
            })
            .await;

        // Then the dashboard shows Dead + the error message.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let (lifecycle, message, _) =
            dashboard_entry(&cell, "discord").expect("entry should exist");
        assert_eq!(lifecycle, ActorLifecycle::Dead);
        assert_eq!(message.as_deref(), Some("Error: 401: invalid token"));
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn discord_error_update_first_still_sets_description() {
        // Given a DashboardActor (simulating missing-token: Error arrives first).
        let harness = TestHarness::new().await;
        let (_actor, cell) = spawn_actor(&harness).await;

        // When publishing an Error update as the very first message.
        harness
            .publish(DiscordStatusUpdate::Error {
                message: "no token configured".to_owned(),
            })
            .await;

        // Then the entry is created with the discord description.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let (_, _, description) = dashboard_entry(&cell, "discord").expect("entry should exist");
        assert_eq!(description.as_deref(), Some("Discord gateway bot [Task]"));
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn browser_binary_verified_writes_chrome_label_to_web_fetch_notes() {
        // Given a DashboardActor.
        let harness = TestHarness::new().await;
        let (_actor, cell) = spawn_actor(&harness).await;

        // When publishing BrowserBinaryVerified for a system Chrome.
        harness
            .publish(BrowserBinaryVerified {
                family: BinaryFamily::Chrome,
                path: Some(std::path::PathBuf::from("/usr/bin/google-chrome")),
                version_major: Some("138".to_owned()),
                fallback_note: None,
            })
            .await;

        // Then the web-fetch row's Notes column carries the backend label.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let (_, message, _) = dashboard_entry(&cell, "web-fetch").expect("entry should exist");
        assert_eq!(
            message.as_deref(),
            Some("Chrome 138 — /usr/bin/google-chrome")
        );
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn browser_binary_verified_writes_bundled_label_to_web_fetch_notes() {
        // Given a DashboardActor.
        let harness = TestHarness::new().await;
        let (_actor, cell) = spawn_actor(&harness).await;

        // When publishing BrowserBinaryVerified for the bundled binary.
        harness
            .publish(BrowserBinaryVerified {
                family: BinaryFamily::Bundled,
                path: None,
                version_major: None,
                fallback_note: Some("No system Chrome/Chromium — using bundled".to_owned()),
            })
            .await;

        // Then the web-fetch row's Notes column shows the bundled label with note.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let (_, message, _) = dashboard_entry(&cell, "web-fetch").expect("entry should exist");
        assert_eq!(
            message.as_deref(),
            Some(
                "No system Chrome/Chromium — using bundled: Chromium (bundled, version undetected)"
            )
        );
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn browser_binary_verified_shows_fallback_version_when_undetected() {
        // Given a DashboardActor.
        let harness = TestHarness::new().await;
        let (_actor, cell) = spawn_actor(&harness).await;

        // When publishing BrowserBinaryVerified for a system Chromium with no version.
        harness
            .publish(BrowserBinaryVerified {
                family: BinaryFamily::Chromium,
                path: Some(std::path::PathBuf::from("/usr/bin/chromium")),
                version_major: None,
                fallback_note: None,
            })
            .await;

        // Then the displayed version falls back to CHROME_MAJOR so it matches the UA.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let (_, message, _) = dashboard_entry(&cell, "web-fetch").expect("entry should exist");
        let expected = format!(
            "Chromium {} (version undetected) — /usr/bin/chromium",
            jinn_web_fetch::stealth::CHROME_MAJOR
        );
        assert_eq!(message.as_deref(), Some(expected.as_str()));
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn browser_binary_verified_does_not_create_phantom_entry() {
        // Given a DashboardActor.
        let harness = TestHarness::new().await;
        let (_actor, cell) = spawn_actor(&harness).await;

        // When publishing BrowserBinaryVerified.
        harness
            .publish(BrowserBinaryVerified {
                family: BinaryFamily::Bundled,
                path: None,
                version_major: None,
                fallback_note: None,
            })
            .await;

        // Then no phantom web-fetch-browser entry is created.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(dashboard_entry(&cell, "web-fetch-browser").is_none());
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn dashboard_nav_command_moves_selection() {
        // Given a DashboardActor with three actor rows.
        let harness = TestHarness::new().await;
        let (_actor, cell) = spawn_actor(&harness).await;
        for name in ["a", "b", "c"] {
            harness
                .publish(ActorStarted {
                    name: name.to_owned(),
                    description: None,
                })
                .await;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // When a DashboardNav::Down message arrives.
        harness.publish(DashboardNav::Down).await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Then the selection moved to index 1.
        assert_eq!(cell.read().selected_index(), 1);
    }
}
