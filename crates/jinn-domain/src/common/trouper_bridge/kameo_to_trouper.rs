//! Kameo → trouper bridge actor.
//!
//! A kameo actor that subscribes to the bus messages consumed by the
//! ported trouper slice actors and republishes each onto its trouper
//! topic. See the parent [`crate::common::trouper_bridge`] module for
//! the topic layout and the crossing messages' `Schema` contracts.

use trouper::system::ActorSystem;

use kameo::actor::Spawn;
use kameo::prelude::{Actor, ActorRef, Context, Message};

use crate::Services;
use crate::common::actor::protocol::event::{ActorShutdownCompleted, ActorStarted, ActorStarting};
use crate::common::actor_deps::ActorDeps;
use crate::common::trouper_bridge::{dashboard_topic, fabric_topic, forward, quake_bar_topic};
use crate::feat::browser_binary_scan::BrowserBinaryVerified;
use crate::feat::dashboard::nav::DashboardNav;
use crate::feat::discord::DiscordStatusUpdate;
use crate::feat::quake_bar::command::SubmitQuakeBarCommand;

/// The bridge actor — kameo bus subscriber, trouper topic publisher.
///
/// Subscribes to exactly the seven message types consumed by the ported
/// trouper slice actors and republishes each onto its topic. Holds the
/// [`ActorSystem`] handle it forwards through.
pub struct KameoToTrouperBridgeActor {
    /// The trouper system the bridge publishes into.
    system: std::sync::Arc<ActorSystem>,
}

/// Dependencies for spawning a [`KameoToTrouperBridgeActor`].
#[derive(Clone)]
pub struct KameoToTrouperBridgeDeps {
    /// Universal actor dependencies (bus subscription handle).
    pub deps: ActorDeps,
    /// The trouper system to forward into.
    pub system: std::sync::Arc<ActorSystem>,
}

impl Actor for KameoToTrouperBridgeActor {
    type Args = KameoToTrouperBridgeDeps;
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
            .subscribe(actor_ref.clone().recipient::<DashboardNav>())
            .await;
        args.deps
            .subscribe(actor_ref.recipient::<SubmitQuakeBarCommand>())
            .await;

        Ok(Self {
            system: args.system,
        })
    }
}

/// Forwards one message: JSON-serialize under its schema and send onto a
/// trouper topic. Topic sends resolve even with zero subscribers (the log
/// entry lands unread), so a returned error means a broken system — warn
/// and continue, matching the bus's fire-and-forget posture.
impl Message<ActorStarting> for KameoToTrouperBridgeActor {
    type Reply = ();

    async fn handle(&mut self, msg: ActorStarting, _ctx: &mut Context<Self, Self::Reply>) {
        forward!(self, msg, fabric_topic(), ActorStarting);
    }
}

impl Message<ActorStarted> for KameoToTrouperBridgeActor {
    type Reply = ();

    async fn handle(&mut self, msg: ActorStarted, _ctx: &mut Context<Self, Self::Reply>) {
        forward!(self, msg, fabric_topic(), ActorStarted);
    }
}

impl Message<ActorShutdownCompleted> for KameoToTrouperBridgeActor {
    type Reply = ();

    async fn handle(&mut self, msg: ActorShutdownCompleted, _ctx: &mut Context<Self, Self::Reply>) {
        forward!(self, msg, fabric_topic(), ActorShutdownCompleted);
    }
}

impl Message<BrowserBinaryVerified> for KameoToTrouperBridgeActor {
    type Reply = ();

    async fn handle(&mut self, msg: BrowserBinaryVerified, _ctx: &mut Context<Self, Self::Reply>) {
        forward!(self, msg, fabric_topic(), BrowserBinaryVerified);
    }
}

impl Message<DiscordStatusUpdate> for KameoToTrouperBridgeActor {
    type Reply = ();

    async fn handle(&mut self, msg: DiscordStatusUpdate, _ctx: &mut Context<Self, Self::Reply>) {
        forward!(self, msg, fabric_topic(), DiscordStatusUpdate);
    }
}

impl Message<DashboardNav> for KameoToTrouperBridgeActor {
    type Reply = ();

    async fn handle(&mut self, msg: DashboardNav, _ctx: &mut Context<Self, Self::Reply>) {
        forward!(self, msg, dashboard_topic(), DashboardNav);
    }
}

impl Message<SubmitQuakeBarCommand> for KameoToTrouperBridgeActor {
    type Reply = ();

    async fn handle(&mut self, msg: SubmitQuakeBarCommand, _ctx: &mut Context<Self, Self::Reply>) {
        forward!(self, msg, quake_bar_topic(), SubmitQuakeBarCommand);
    }
}

/// Spawns the bridge on the root supervisor and waits for startup.
///
/// The bridge must be subscribed to the bus before any ported slice
/// actor activates — lifecycle announcements published afterwards are
/// what the dashboard's rows fold.
///
/// # Errors
///
/// Propagates a spawn failure from the kameo supervisor.
pub async fn spawn_kameo_to_trouper(
    services: &Services,
) -> kameo::prelude::ActorRef<KameoToTrouperBridgeActor> {
    let actor = KameoToTrouperBridgeActor::supervise(
        &services.root_supervisor,
        KameoToTrouperBridgeDeps {
            deps: ActorDeps {
                services: services.clone(),
            },
            system: services.trouper_system.clone(),
        },
    )
    .restart_policy(kameo::supervision::RestartPolicy::Never)
    .spawn()
    .await;
    actor.wait_for_startup().await;
    actor
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        clippy::panic,
        clippy::unwrap_used,
        clippy::indexing_slicing,
        reason = "test code"
    )]
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use trouper::actor::MsgHandler;
    use trouper::context::MsgCtx;
    use trouper::registry::RegistryError;
    use trouper::schema::Schema;
    use trouper::types::{ActorPath, SchemaId};

    /// Every crossing message type round-trips through serde and carries
    /// a stable version-1 schema id.
    #[rstest::rstest]
    #[test]
    fn crossing_messages_roundtrip_with_stable_schema_ids() {
        // Given one instance of each crossing message type.
        let quake = SubmitQuakeBarCommand {
            text: "hello".to_owned(),
        };
        let nav = DashboardNav::Down;
        let starting = ActorStarting {
            name: "llm".to_owned(),
            description: None,
        };
        let started = ActorStarted {
            name: "llm".to_owned(),
            description: Some("LlmActor".to_owned()),
        };
        let shutdown = ActorShutdownCompleted {
            name: "llm".to_owned(),
        };
        let browser = BrowserBinaryVerified {
            family: crate::feat::browser_binary_scan::BinaryFamily::Chrome,
            path: Some(std::path::PathBuf::from("/usr/bin/chrome")),
            version_major: Some("138".to_owned()),
            fallback_note: None,
        };
        let discord = DiscordStatusUpdate::Connected;

        // When round-tripping each through serde and naming its schema.
        let cases: Vec<(SchemaId, serde_json::Value, serde_json::Value)> = vec![
            (
                SubmitQuakeBarCommand::schema_id(),
                serde_json::to_value(&quake).unwrap(),
                serde_json::to_value(&roundtrip(&quake)).unwrap(),
            ),
            (
                DashboardNav::schema_id(),
                serde_json::to_value(&nav).unwrap(),
                serde_json::to_value(&roundtrip(&nav)).unwrap(),
            ),
            (
                ActorStarting::schema_id(),
                serde_json::to_value(&starting).unwrap(),
                serde_json::to_value(&roundtrip(&starting)).unwrap(),
            ),
            (
                ActorStarted::schema_id(),
                serde_json::to_value(&started).unwrap(),
                serde_json::to_value(&roundtrip(&started)).unwrap(),
            ),
            (
                ActorShutdownCompleted::schema_id(),
                serde_json::to_value(&shutdown).unwrap(),
                serde_json::to_value(&roundtrip(&shutdown)).unwrap(),
            ),
            (
                BrowserBinaryVerified::schema_id(),
                serde_json::to_value(&browser).unwrap(),
                serde_json::to_value(&roundtrip(&browser)).unwrap(),
            ),
            (
                DiscordStatusUpdate::schema_id(),
                serde_json::to_value(&discord).unwrap(),
                serde_json::to_value(&roundtrip(&discord)).unwrap(),
            ),
        ];

        // Then every payload survived the roundtrip unchanged, and every
        // schema id is `<TypeName>@1`.
        for (id, original, roundtripped) in cases {
            assert_eq!(original, roundtripped, "payload changed for {id}");
            assert_eq!(
                id.to_string(),
                format!("{}@1", id.to_string().split('@').next().unwrap()),
                "schema id must be name@1 for {id}"
            );
        }
    }

    fn roundtrip<M>(msg: &M) -> M
    where
        M: trouper::schema::Schema + serde::Serialize + serde::de::DeserializeOwned,
    {
        serde_json::from_value(serde_json::to_value(msg).unwrap()).unwrap()
    }

    /// A probe service actor counting the fabric envelopes it receives.
    struct ProbeActor {
        hits: Arc<AtomicUsize>,
    }

    impl trouper::actor::ServiceActor for ProbeActor {
        async fn start(
            _args: &serde_json::Value,
        ) -> Result<Self, trouper::error_stack::Report<RegistryError>> {
            unreachable!("spawned via start_with; start is never called")
        }
    }

    impl MsgHandler<ActorStarted> for ProbeActor {
        async fn handle(&mut self, _msg: ActorStarted, _ctx: &mut MsgCtx<'_>) {
            self.hits.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// A bus publish lands on the trouper topic as a schema-tagged JSON
    /// envelope the probe's subscription receives.
    #[rstest::rstest]
    #[tokio::test]
    async fn bridge_translates_bus_publish_to_topic_envelope() {
        // Given a trouper system, a bridge forwarding the bus into it, and
        // a probe actor subscribed to the fabric topic.
        let services = Services::new_fake().await;
        spawn_kameo_to_trouper(&services).await;
        let hits = Arc::new(AtomicUsize::new(0));
        let system = services.trouper_system.clone();
        trouper::builder::spawn_service_builder::<ProbeActor>(&system)
            .at(ActorPath::new("probe"))
            .start_with({
                let hits = hits.clone();
                move || Box::pin(async move { Ok(ProbeActor { hits }) })
            })
            .handles::<ActorStarted>()
            .start();
        system
            .subscribe(&ActorPath::new("probe"), &fabric_topic(), None)
            .expect("probe subscribes to the fabric topic");

        // When publishing an ActorStarted on the kameo bus.
        services
            .bus
            .publish(ActorStarted {
                name: "llm".to_owned(),
                description: None,
            })
            .await;

        // Then the probe receives exactly one envelope.
        for _ in 0..200 {
            if hits.load(Ordering::SeqCst) > 0 {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        panic!("probe never received the forwarded envelope");
    }
}
