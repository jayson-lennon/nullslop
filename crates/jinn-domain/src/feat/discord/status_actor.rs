//! The Discord status actor — a pure translator.
//!
//! Drains a kanal channel fed by the Discord gateway task and translates each
//! [`DiscordStatusUpdate`] into a generic
//! [`ServiceStatusUpdate`](crate::feat::dashboard::ServiceStatusUpdate) bus
//! event. It owns no application state and never touches `frontend.dashboard`
//! — the [`DashboardActor`] is the single sink that subscribes to
//! [`ServiceStatusUpdate`] and writes the dashboard.
//!
//! Keeping the gateway's kanal channel intact (it is a tokio task, not a kameo
//! actor), this actor only changes the *destination* of its updates: from a
//! direct dashboard write to a bus publication, in the dashboard's generic
//! vocabulary.

use kameo::actor::ActorRef;
use kameo::prelude::Actor;

use crate::common::actor_deps::ActorDeps;
use crate::feat::dashboard::{ActorLifecycle, ServiceStatusUpdate};

/// The dashboard row name for the discord entry.
const DISCORD_ACTOR_NAME: &str = "discord";

/// The description attached to the discord dashboard row.
const DISCORD_DESCRIPTION: &str = "Discord gateway bot [Task]";

/// Discord bot-specific connection status, reported by the gateway task.
///
/// Rendered as the free-form third column of the discord dashboard entry.
/// Other services leave `status_message` empty unless they publish their own
/// [`ServiceStatusUpdate`].
#[derive(Debug, Clone)]
pub enum DiscordStatusUpdate {
    /// The gateway is attempting to connect to Discord.
    Connecting,
    /// The gateway received its `ready` event — the bot is online.
    Connected,
    /// The websocket dropped mid-session.
    Disconnected,
    /// The gateway hit a fatal error (auth failure, unresolvable disconnect).
    Error {
        /// Human-readable reason (e.g. "401: invalid bot token").
        message: String,
    },
}

impl DiscordStatusUpdate {
    /// Renders the update into the dashboard `status_message` string.
    #[must_use]
    pub fn display_message(&self) -> &'static str {
        match self {
            Self::Connecting => "Connecting",
            Self::Connected => "Connected",
            Self::Disconnected => "Disconnected",
            Self::Error { .. } => "Error",
        }
    }

    /// Returns the full human-readable detail (for the `Error` variant).
    #[must_use]
    pub fn full_message(&self) -> String {
        match self {
            Self::Error { message } => format!("Error: {message}"),
            other => other.display_message().to_owned(),
        }
    }

    /// Translates this connection state into the dashboard's generic
    /// status event vocabulary.
    ///
    /// Mapping:
    /// - `Connecting` → row `Starting` + description
    /// - `Connected` → row `Running` + description
    /// - `Error` → row `Dead` + description (so the row is described even
    ///   when the error arrives first, e.g. missing token)
    /// - `Disconnected` → status message only; the row lifecycle stays
    ///   untouched (`Running`), and the description is preserved.
    #[must_use]
    pub fn to_service_update(&self) -> ServiceStatusUpdate {
        let (lifecycle, with_description) = match self {
            // The gateway task is not a kameo actor, so it doesn't emit
            // ActorStarting — ensure the entry exists with a description.
            Self::Connecting => (Some(ActorLifecycle::Starting), true),
            // The gateway task is not a kameo actor, so it doesn't emit
            // ActorStarted — mark the row running here.
            Self::Connected => (Some(ActorLifecycle::Running), true),
            // The description is a constant for the discord entry; attach it
            // on creation even when Error arrives first (e.g. missing token).
            Self::Error { .. } => (Some(ActorLifecycle::Dead), true),
            // Disconnected only updates the status message — the lifecycle
            // is driven by the other variants.
            Self::Disconnected => (None, false),
        };
        ServiceStatusUpdate {
            name: DISCORD_ACTOR_NAME.to_owned(),
            description: with_description.then(|| DISCORD_DESCRIPTION.to_owned()),
            lifecycle,
            status_message: Some(self.full_message()),
        }
    }
}

/// The Discord status actor — a pure translator.
///
/// Subscribes to nothing. Spawns a background drain loop that reads each
/// [`DiscordStatusUpdate`] from the kanal channel, translates it into a
/// [`ServiceStatusUpdate`], and publishes that on the bus. The
/// [`DashboardActor`] consumes it from there.
pub struct DiscordStatusActor;

/// Dependencies for [`DiscordStatusActor`].
#[derive(Clone)]
pub struct DiscordStatusActorDeps {
    /// Universal actor dependencies (bus publish handle).
    pub deps: ActorDeps,
    /// Receiver half of the kanal channel fed by the Discord gateway.
    pub status_rx: kanal::AsyncReceiver<DiscordStatusUpdate>,
}

impl Actor for DiscordStatusActor {
    type Args = DiscordStatusActorDeps;
    type Error = kameo::error::Infallible;

    async fn on_start(args: Self::Args, _actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        // Spawn the background drain loop: read each gateway update, translate
        // it, and publish it on the bus for the DashboardActor.
        let deps = args.deps;
        tokio::spawn(drain_status_channel(args.status_rx, deps));
        Ok(Self)
    }
}

/// Background drain loop: reads discord status updates from the kanal channel,
/// translates them into generic status events, and republishes them on the bus.
async fn drain_status_channel(rx: kanal::AsyncReceiver<DiscordStatusUpdate>, deps: ActorDeps) {
    while let Ok(update) = rx.recv().await {
        let () = deps.services.bus.publish(update.to_service_update()).await;
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        reason = "test code"
    )]
    use super::*;
    use crate::common::app_state::AppState;
    use crate::common::bus::test_harness::TestHarness;
    use crate::common::state::State;
    use crate::feat::dashboard::dashboard_actor::{DashboardActor, DashboardActorDeps};
    use kameo::actor::Spawn;

    async fn spawn_translator(
        harness: &TestHarness,
    ) -> (
        kanal::Sender<DiscordStatusUpdate>,
        ActorRef<DiscordStatusActor>,
    ) {
        let (tx, rx) = kanal::unbounded::<DiscordStatusUpdate>();
        let actor = DiscordStatusActor::spawn(DiscordStatusActorDeps {
            deps: harness.actor_deps().await,
            status_rx: rx.to_async(),
        });
        actor.wait_for_startup().await;
        (tx, actor)
    }

    #[rstest::rstest]
    #[case::connecting(DiscordStatusUpdate::Connecting, Some(ActorLifecycle::Starting), true)]
    #[case::connected(DiscordStatusUpdate::Connected, Some(ActorLifecycle::Running), true)]
    #[case::disconnected(DiscordStatusUpdate::Disconnected, None, false)]
    fn to_service_update_maps_lifecycle_and_description_per_variant(
        #[case] update: DiscordStatusUpdate,
        #[case] expected_lifecycle: Option<ActorLifecycle>,
        #[case] with_description: bool,
    ) {
        // Given a gateway connection state.

        // When translating it into the generic dashboard vocabulary.
        let update = update.to_service_update();

        // Then the row name is the discord entry.
        assert_eq!(update.name, DISCORD_ACTOR_NAME);
        // And the lifecycle mapping matches the connection state.
        assert_eq!(update.lifecycle, expected_lifecycle);
        // And the description follows the variant's attachment rule.
        assert_eq!(update.description.is_some(), with_description);
        // And a status message is always present.
        assert!(update.status_message.is_some());
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn translated_updates_reach_the_bus_with_discord_description() {
        // Given a DiscordStatusActor (translator) recording bus publications.
        let harness = TestHarness::new().await;
        let (tx, _actor) = spawn_translator(&harness).await;
        let recorder = harness.spawn_recorder::<ServiceStatusUpdate>().await;

        // When the gateway sends a Connected update down the kanal channel.
        let _ = tx.send(DiscordStatusUpdate::Connected);

        // Then the bus carries the generic update for the discord row.
        let messages = crate::common::bus::test_harness::await_recorded(
            &recorder,
            1,
            std::time::Duration::from_secs(2),
        )
        .await;
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].name, DISCORD_ACTOR_NAME);
        assert_eq!(messages[0].lifecycle, Some(ActorLifecycle::Running));
        assert_eq!(
            messages[0].description.as_deref(),
            Some(DISCORD_DESCRIPTION)
        );
        assert_eq!(messages[0].status_message.as_deref(), Some("Connected"));
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn republishes_kanal_update_on_the_bus() {
        // Given a DiscordStatusActor (translator) and a DashboardActor (consumer).
        let harness = TestHarness::new().await;
        let (tx, _actor) = spawn_translator(&harness).await;
        let state = State::new(AppState::default());
        let dash = DashboardActor::spawn(DashboardActorDeps {
            deps: harness.actor_deps().await,
            state: state.clone(),
            cap: crate::common::tcaps::mint::mint_frontend_cap(),
        });
        dash.wait_for_startup().await;

        // When the gateway sends a Connected update down the kanal channel.
        let _ = tx.send(DiscordStatusUpdate::Connected);

        // Then the dashboard (fed only via the bus) shows the discord entry
        // as Running with the Connected message — proving the translator
        // republished the update and wrote nothing itself.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let (lifecycle, message) = {
            let g = state.read();
            let actors = g.frontend.dashboard.actors();
            let discord = actors
                .iter()
                .find(|e| e.name == "discord")
                .expect("discord entry exists via bus republish");
            (discord.lifecycle, discord.status_message.clone())
        };
        assert_eq!(lifecycle, ActorLifecycle::Running);
        assert_eq!(message.as_deref(), Some("Connected"));
    }
}
