//! The Discord status actor — a pure translator.
//!
//! Drains a kanal channel fed by the Discord gateway task and republishes each
//! [`DiscordStatusUpdate`] on the bus. It owns no application state and never
//! touches `frontend.dashboard` — the [`DashboardActor`] is the single sink
//! that subscribes to [`DiscordStatusUpdate`] and writes the dashboard.
//!
//! Keeping the gateway's kanal channel intact (it is a tokio task, not a kameo
//! actor), this actor only changes the *destination* of its updates: from a
//! direct dashboard write to a bus publication.

use kameo::actor::ActorRef;
use kameo::prelude::Actor;

use crate::common::actor_deps::ActorDeps;
use crate::common::bus::BusMessage;

/// Discord bot-specific connection status, reported by the gateway task.
///
/// Rendered as the free-form third column of the discord dashboard entry.
/// Other actors leave `status_message` empty; only discord populates this.
///
/// This type serves double duty: it is both the kanal message (gateway →
/// [`DiscordStatusActor`]) and the bus message ([`DiscordStatusActor`] →
/// [`DashboardActor`](crate::feat::dashboard::dashboard_actor::DashboardActor)).
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

impl BusMessage for DiscordStatusUpdate {}

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
}

/// The Discord status actor — a pure translator.
///
/// Subscribes to nothing. Spawns a background drain loop that reads each
/// [`DiscordStatusUpdate`] from the kanal channel and publishes it on the bus.
/// The [`DashboardActor`] consumes it from there.
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
        // Spawn the background drain loop: read each gateway update and
        // republish it on the bus so the DashboardActor can consume it.
        let deps = args.deps;
        tokio::spawn(drain_status_channel(args.status_rx, deps));
        Ok(Self)
    }
}
/// Background drain loop: reads discord status updates from the kanal channel
/// and republishes them on the bus.
async fn drain_status_channel(rx: kanal::AsyncReceiver<DiscordStatusUpdate>, deps: ActorDeps) {
    while let Ok(update) = rx.recv().await {
        let () = deps.services.bus.publish(update).await;
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic, reason = "test code")]
    use super::*;
    use crate::common::bus::test_harness::TestHarness;
    use crate::common::slices::Slices;
    use crate::feat::dashboard::ActorLifecycle;
    use crate::feat::dashboard::DashboardState;
    use crate::feat::dashboard::dashboard_actor::{DashboardActor, DashboardActorDeps};
    use crate::feat::dashboard::dashboard_slot;
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
    #[tokio::test]
    async fn republishes_kanal_update_on_the_bus() {
        // Given a DiscordStatusActor (translator) and a DashboardActor (consumer).
        let harness = TestHarness::new().await;
        let (tx, _actor) = spawn_translator(&harness).await;
        let slices = Slices::new();
        let cell = slices
            .register(dashboard_slot(), DashboardState::new())
            .expect("fresh registry");
        let dash = DashboardActor::spawn(DashboardActorDeps {
            deps: harness.actor_deps().await,
            cell: cell.clone(),
        });
        dash.wait_for_startup().await;

        // When the gateway sends a Connected update down the kanal channel.
        let _ = tx.send(DiscordStatusUpdate::Connected);

        // Then the dashboard (fed only via the bus) shows the discord entry
        // as Running with the Connected message — proving the translator
        // republished the update and wrote nothing itself.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let (lifecycle, message) = {
            let s = cell.read();
            let actors = s.actors();
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
