//! The Discord status actor — the connection authority.
//!
//! Drains a kanal channel fed by the Discord gateway task and republishes each
//! [`DiscordStatusUpdate`] on the bus, folding the authoritative connection
//! fact into discord's own slice cell ([`discord_connection_slot`]). The
//! [`DashboardActor`] subscribes to the same event for display only.
//!
//! Keeping the gateway's kanal channel intact (it is a tokio task, not a kameo
//! actor), this actor only changes the *destination* of its updates: from a
//! direct dashboard write to bus publication + own-cell fold.

use kameo::actor::ActorRef;
use kameo::prelude::Actor;

use crate::common::actor_deps::ActorDeps;
use crate::common::bus::BusMessage;
use crate::common::slices::SlotKey;
use crate::common::slices::TypedCell;

/// Discord bot-specific connection status, reported by the gateway task.
///
/// Rendered as the free-form third column of the discord dashboard entry.
/// Other actors leave `status_message` empty; only discord populates this.
///
/// This type serves triple duty: it is the kanal message (gateway →
/// [`DiscordStatusActor`]), the bus message ([`DiscordStatusActor`] →
/// the dashboard canvas actor), and the canvas topic payload (the
/// kameo→trouper bridge serializes it onto `jinn.fabric`) — hence the
/// serde derives.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    /// The dashboard entry name for the discord gateway. Discord facts
    /// live here, not in consumers — the dashboard folds this identity
    /// straight from the event.
    #[must_use]
    pub fn entry_name(&self) -> &'static str {
        "discord"
    }

    /// The dashboard entry description for the discord gateway.
    #[must_use]
    pub fn entry_description(&self) -> &'static str {
        "Discord gateway bot [Task]"
    }

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

/// Discord's own connection fact, folded by [`DiscordStatusActor`].
///
/// The single source of truth for "is the bot connected": feature gates
/// (e.g. thread creation) read this cell instead of greping the
/// dashboard's actor table. One writer — the status actor's fold.
#[derive(Debug, Clone)]
pub struct ConnectionState {
    /// Whether the gateway considers the bot online.
    pub connected: bool,
    /// Optional detail (e.g. the error message while disconnected).
    pub detail: Option<String>,
}

/// Discord's connection cell slot in the
/// [`Slices`](crate::common::slices::Slices) facade.
///
/// Canonical key shared by wiring (which mints the cell), the status
/// actor (which folds it), and feature gates (which read it).
#[must_use]
pub fn discord_connection_slot() -> SlotKey {
    SlotKey::builtin("discord", "connection")
}

/// The Discord status actor — the connection authority.
///
/// Subscribes to nothing. Spawns a background drain loop that reads each
/// [`DiscordStatusUpdate`] from the kanal channel, folds it into the
/// connection cell, and publishes it on the bus (the [`DashboardActor`]
/// consumes it from there for display only).
pub struct DiscordStatusActor;

/// Dependencies for [`DiscordStatusActor`].
#[derive(Clone)]
pub struct DiscordStatusActorDeps {
    /// Universal actor dependencies (bus publish handle).
    pub deps: ActorDeps,
    /// Receiver half of the kanal channel fed by the Discord gateway.
    pub status_rx: kanal::AsyncReceiver<DiscordStatusUpdate>,
    /// The write handle for discord's connection cell — this actor is
    /// its single writer.
    pub cell: TypedCell<ConnectionState>,
}

impl Actor for DiscordStatusActor {
    type Args = DiscordStatusActorDeps;
    type Error = kameo::error::Infallible;

    async fn on_start(args: Self::Args, _actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        // Spawn the background drain loop: read each gateway update,
        // fold it into the connection cell, and republish it on the bus
        // so the DashboardActor can consume it.
        let deps = args.deps;
        tokio::spawn(drain_status_channel(args.status_rx, deps, args.cell));
        Ok(Self)
    }
}
/// Background drain loop: reads discord status updates from the kanal
/// channel, folds the connection fact into the cell, and republishes
/// them on the bus.
async fn drain_status_channel(
    rx: kanal::AsyncReceiver<DiscordStatusUpdate>,
    deps: ActorDeps,
    cell: TypedCell<ConnectionState>,
) {
    while let Ok(update) = rx.recv().await {
        cell.update(|state| fold_connection(state, &update));
        let () = deps.services.bus.publish(update).await;
    }
}

/// Applies an update to the connection cell state.
fn fold_connection(state: &mut ConnectionState, update: &DiscordStatusUpdate) {
    match update {
        DiscordStatusUpdate::Connecting => {
            state.connected = false;
            state.detail = Some("Connecting".to_owned());
        }
        DiscordStatusUpdate::Connected => {
            state.connected = true;
            state.detail = None;
        }
        DiscordStatusUpdate::Disconnected => {
            state.connected = false;
            state.detail = Some("Disconnected".to_owned());
        }
        DiscordStatusUpdate::Error { message } => {
            state.connected = false;
            state.detail = Some(message.clone());
        }
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
    use crate::feat::dashboard::canvas_actor::DashboardCanvasActor;
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

    #[rstest::rstest]
    #[tokio::test]
    async fn republishes_kanal_update_on_the_bus() {
        // Given a DiscordStatusActor and a DashboardActor (display consumer).
        let harness = TestHarness::new().await;
        let slices = Slices::new();
        let connection = slices
            .register(
                discord_connection_slot(),
                ConnectionState {
                    connected: false,
                    detail: None,
                },
            )
            .expect("fresh registry");
        let (tx, _actor) = spawn_translator(&harness, connection).await;
        let cell = slices
            .register(dashboard_slot(), DashboardState::new())
            .expect("fresh registry");
        // The dashboard display consumer runs on the canvas runtime, fed
        // by the bridge over the harness bus.
        let services = harness.services().await;
        crate::common::trouper_bridge::spawn_kameo_to_trouper(&services).await;
        DashboardCanvasActor::spawn(&services.trouper_system, cell.clone());

        // When the gateway sends a Connected update down the kanal channel.
        let _ = tx.send(DiscordStatusUpdate::Connected);

        // Then the dashboard (fed only via the bus) shows the discord entry
        // as Running with the Connected message — proving the actor
        // republished the update and the dashboard writes display only.
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
    async fn folds_kanal_update_into_connection_cell() {
        // Given a DiscordStatusActor with its connection cell.
        let harness = TestHarness::new().await;
        let slices = Slices::new();
        let connection = slices
            .register(
                discord_connection_slot(),
                ConnectionState {
                    connected: false,
                    detail: None,
                },
            )
            .expect("fresh registry");
        let (tx, _actor) = spawn_translator(&harness, connection.clone()).await;

        // When the gateway sends Error then Connected updates.
        let _ = tx.send(DiscordStatusUpdate::Error {
            message: "401: invalid bot token".to_owned(),
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let _ = tx.send(DiscordStatusUpdate::Connected);
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Then the cell reflects the authoritative fact, latest wins.
        let s = connection.read();
        assert!(s.connected, "connected update must set the flag");
        // And the detail cleared on success.
        assert_eq!(s.detail, None);
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn error_update_leaves_cell_disconnected_with_detail() {
        // Given a DiscordStatusActor with its connection cell.
        let harness = TestHarness::new().await;
        let slices = Slices::new();
        let connection = slices
            .register(
                discord_connection_slot(),
                ConnectionState {
                    connected: false,
                    detail: None,
                },
            )
            .expect("fresh registry");
        let (tx, _actor) = spawn_translator(&harness, connection.clone()).await;

        // When the gateway reports a fatal error.
        let _ = tx.send(DiscordStatusUpdate::Error {
            message: "401: invalid bot token".to_owned(),
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Then the cell stays disconnected and carries the reason.
        let s = connection.read();
        assert!(!s.connected);
        assert_eq!(s.detail.as_deref(), Some("401: invalid bot token"));
    }
}
