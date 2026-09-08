//! The Discord status actor — a pure translator.
//!
//! Drains a kanal channel fed by the Discord gateway task and republishes each
//! [`DiscordStatusUpdate`] on the bus. It owns no application state — the
//! [`DashboardActor`](crate::feat::dashboard::dashboard_actor::DashboardActor)
//! subscribes to [`DiscordStatusUpdate`] and folds it into the dashboard for
//! display only; the authoritative `is connected` fact lives in the discord
//! connection cell this actor folds alongside the republish.
//!
//! Keeping the gateway's kanal channel intact (it is a tokio task, not a kameo
//! actor), this actor only changes the *destination* of its updates: from a
//! direct dashboard write to a bus publication.

use kameo::actor::ActorRef;
use kameo::prelude::Actor;

use jinn_slices::SlotKey;
use jinn_slices::TypedCell;

use crate::common::actor_deps::ActorDeps;
use crate::common::bus::BusMessage;

/// The discord connection cell's slot in the
/// [`Slices`](jinn_slices::Slices) registry.
///
/// Canonical key shared by the status actor (which folds connection
/// state into it) and the thread-creation gate (which reads it).
#[must_use]
pub fn discord_connection_slot() -> SlotKey {
    SlotKey::builtin("discord", "connection")
}

/// The authoritative bot-connection fact, folded by the status actor.
///
/// Distinct from the dashboard's `status_message` display fold: this is
/// the state other features consult (e.g. the thread-creation gate), so
/// a removed dashboard cannot degrade discord's own behavior.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionState {
    /// `true` once the gateway has reported `ready`.
    pub connected: bool,
    /// Latest human-readable detail (error text, current sub-state).
    pub detail: Option<String>,
}

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
/// [`DiscordStatusUpdate`] from the kanal channel, publishes it on the bus,
/// and folds the authoritative connection state into the discord
/// connection cell. The
/// [`DashboardActor`](crate::feat::dashboard::dashboard_actor::DashboardActor)
/// consumes the bus event for display only.
pub struct DiscordStatusActor;

/// Dependencies for [`DiscordStatusActor`].
#[derive(Clone)]
pub struct DiscordStatusActorDeps {
    /// Universal actor dependencies (bus publish handle).
    pub deps: ActorDeps,
    /// Receiver half of the kanal channel fed by the Discord gateway.
    pub status_rx: kanal::AsyncReceiver<DiscordStatusUpdate>,
    /// The discord connection cell — the one handle minted at wiring;
    /// this actor is its single writer.
    pub cell: TypedCell<ConnectionState>,
}

impl Actor for DiscordStatusActor {
    type Args = DiscordStatusActorDeps;
    type Error = kameo::error::Infallible;

    async fn on_start(args: Self::Args, _actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        // Spawn the background drain loop: read each gateway update,
        // republish it on the bus, and fold the connection fact into the
        // cell.
        let deps = args.deps;
        tokio::spawn(drain_status_channel(
            args.status_rx,
            deps,
            args.cell,
        ));
        Ok(Self)
    }
}

/// Background drain loop: reads discord status updates from the kanal
/// channel, republishes them on the bus, and folds the connection fact
/// into the cell.
async fn drain_status_channel(
    rx: kanal::AsyncReceiver<DiscordStatusUpdate>,
    deps: ActorDeps,
    cell: TypedCell<ConnectionState>,
) {
    while let Ok(update) = rx.recv().await {
        let () = deps.services.bus.publish(update.clone()).await;
        cell.update(|state| *state = ConnectionState::from(&update));
    }
}

impl From<&DiscordStatusUpdate> for ConnectionState {
    fn from(update: &DiscordStatusUpdate) -> Self {
        match update {
            DiscordStatusUpdate::Connecting => Self {
                connected: false,
                detail: Some("Connecting".to_owned()),
            },
            DiscordStatusUpdate::Connected => Self {
                connected: true,
                detail: None,
            },
            DiscordStatusUpdate::Disconnected => Self {
                connected: false,
                detail: Some("Disconnected".to_owned()),
            },
            DiscordStatusUpdate::Error { message } => Self {
                connected: false,
                detail: Some(message.clone()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic, reason = "test code")]
    use super::*;
    use crate::common::bus::test_harness::TestHarness;
    use jinn_slices::Slices;
    use crate::feat::dashboard::ActorLifecycle;
    use crate::feat::dashboard::DashboardState;
    use crate::feat::dashboard::dashboard_actor::{DashboardActor, DashboardActorDeps};
    use crate::feat::dashboard::dashboard_slot;
    use kameo::actor::Spawn;

    async fn spawn_translator(
        harness: &TestHarness,
        cell: TypedCell<ConnectionState>,
    ) -> (
        kanal::Sender<DiscordStatusUpdate>,
        ActorRef<DiscordStatusActor>,
    ) {
        let (tx, rx) = kanal::unbounded::<DiscordStatusUpdate>();
        let actor = DiscordStatusActor::spawn(DiscordStatusActorDeps {
            deps: harness.actor_deps().await,
            status_rx: rx.to_async(),
            cell,
        });
        actor.wait_for_startup().await;
        (tx, actor)
    }

    async fn connection_cell() -> TypedCell<ConnectionState> {
        let slices = Slices::new();
        slices
            .register(
                discord_connection_slot(),
                ConnectionState {
                    connected: false,
                    detail: None,
                },
            )
            .expect("fresh registry")
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn republishes_kanal_update_on_the_bus() {
        // Given a DiscordStatusActor (translator) and a DashboardActor (consumer).
        let harness = TestHarness::new().await;
        let (tx, _actor) = spawn_translator(&harness, connection_cell().await).await;
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

    #[rstest::rstest]
    #[tokio::test]
    async fn connected_update_folds_into_the_connection_cell() {
        // Given a DiscordStatusActor over a fresh connection cell.
        let harness = TestHarness::new().await;
        let cell = connection_cell().await;
        let (tx, _actor) = spawn_translator(&harness, cell.clone()).await;

        // When the gateway reports Connected.
        let _ = tx.send(DiscordStatusUpdate::Connected);
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Then the cell reports connected.
        assert!(cell.read().connected);
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn error_update_carries_detail_and_disconnected_state() {
        // Given a DiscordStatusActor over a fresh connection cell.
        let harness = TestHarness::new().await;
        let cell = connection_cell().await;
        let (tx, _actor) = spawn_translator(&harness, cell.clone()).await;

        // When the gateway reports a fatal error.
        let _ = tx.send(DiscordStatusUpdate::Error {
            message: "401: invalid bot token".to_owned(),
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Then the cell is disconnected and carries the error detail.
        let state = cell.read();
        assert!(!state.connected);
        assert_eq!(state.detail.as_deref(), Some("401: invalid bot token"));
    }
}
