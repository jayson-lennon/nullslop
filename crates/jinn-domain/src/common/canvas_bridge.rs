//! Kameo → actor-canvas bridge.
//!
//! Jinn's kameo message bus and the actor-canvas `ActorSystem` are two
//! separate fabrics. The slice actors that have been ported to
//! actor-canvas (dashboard, quake-bar) can no longer subscribe to kameo
//! bus messages directly, so this module is the one translation seam:
//! a kameo actor that subscribes to exactly the messages the ported
//! slices consume and republishes each one onto its canvas topic, where
//! the ported actors' topic subscriptions pick it up.
//!
//! Topic layout (see [`topics`]):
//!
//! - `jinn.fabric` — actor lifecycle events, browser binary resolution,
//!   and discord status (the dashboard's cross-actor inputs).
//! - `jinn.dashboard` — dashboard keyboard navigation.
//! - `jinn.quake-bar` — quake bar submit commands.
//!
//! Payloads cross as JSON under each message's [`Schema`] contract;
//! the `Schema` impls for the seven crossing types live here (schema
//! descriptors are transport metadata — they belong with the bridge
//! that mints envelopes, not with the domain types themselves).
//!
//! Delivery semantics match the bus's `BestEffort` strategy:
//! fire-and-forget, a warn log on unroutable sends, no retry.

use actor_runtime::envelope::Event;
use actor_runtime::schema::{FieldDef, FieldTy, Schema, SchemaDef, SchemaKind};
use actor_runtime::system::ActorSystem;
use actor_runtime::types::Topic;

use kameo::actor::Spawn;
use kameo::prelude::{Actor, ActorRef, Context, Message};

use crate::Services;
use crate::common::actor::protocol::event::{ActorShutdownCompleted, ActorStarted, ActorStarting};
use crate::common::actor_deps::ActorDeps;
use crate::feat::browser_binary_scan::BrowserBinaryVerified;
use crate::feat::dashboard::nav::DashboardNav;
use crate::feat::discord::DiscordStatusUpdate;
use crate::feat::quake_bar::command::SubmitQuakeBarCommand;

/// Canvas topic names the bridge publishes onto.
pub mod topics {
    /// Actor lifecycle + cross-actor status events (dashboard input).
    pub const FABRIC: &str = "jinn.fabric";
    /// Dashboard keyboard navigation.
    pub const DASHBOARD: &str = "jinn.dashboard";
    /// Quake bar submit commands.
    pub const QUAKE_BAR: &str = "jinn.quake-bar";
}

/// The fabric topic (`jinn.fabric`) as a [`Topic`].
#[must_use]
pub fn fabric_topic() -> Topic {
    Topic::new(topics::FABRIC)
}

/// The dashboard topic (`jinn.dashboard`) as a [`Topic`].
#[must_use]
pub fn dashboard_topic() -> Topic {
    Topic::new(topics::DASHBOARD)
}

/// The quake-bar topic (`jinn.quake-bar`) as a [`Topic`].
#[must_use]
pub fn quake_bar_topic() -> Topic {
    Topic::new(topics::QUAKE_BAR)
}

/// Implements [`Schema`] for a crossing message type.
///
/// `name` mirrors the Rust type name so canvas exports read the same on
/// both sides of the bridge; all crossing schemas are version 1.
macro_rules! impl_schema {
    ($ty:ty, $name:literal, $kind:expr, description: $desc:literal, fields: [$($field:literal => $fty:expr),* $(,)?]) => {
        impl Schema for $ty {
            fn schema_def() -> SchemaDef {
                SchemaDef {
                    name: $name.to_owned(),
                    version: 1,
                    kind: $kind,
                    fields: vec![$(FieldDef::required($field, $fty)),*],
                    description: Some($desc.to_owned()),
                }
            }
        }
    };
}

impl_schema!(SubmitQuakeBarCommand, "SubmitQuakeBarCommand", SchemaKind::Command,
    description: "Submit the current quake bar input into the command log.",
    fields: ["text" => FieldTy::Str]);

impl_schema!(DashboardNav, "DashboardNav", SchemaKind::Command,
    description: "Move the dashboard's selection cursor (enum payload).",
    fields: []);

impl_schema!(ActorStarting, "ActorStarting", SchemaKind::Event,
    description: "An actor is starting up.",
    fields: ["name" => FieldTy::Str]);

impl_schema!(ActorStarted, "ActorStarted", SchemaKind::Event,
    description: "An actor has finished starting up.",
    fields: ["name" => FieldTy::Str]);

impl_schema!(ActorShutdownCompleted, "ActorShutdownCompleted", SchemaKind::Event,
    description: "An actor has completed shutdown.",
    fields: ["name" => FieldTy::Str]);

impl_schema!(BrowserBinaryVerified, "BrowserBinaryVerified", SchemaKind::Event,
    description: "The configured browser binary has been resolved (enum + paths in payload).",
    fields: ["family" => FieldTy::Str]);

impl_schema!(DiscordStatusUpdate, "DiscordStatusUpdate", SchemaKind::Event,
    description: "Discord gateway connection status (enum payload).",
    fields: []);

/// Builds a canvas [`Event`] from a crossing message.
///
/// Serialization cannot fail for these types (plain structs/enums), so a
/// failure degrades to a null payload rather than a panic in an actor
/// handler.
fn event_of<M: Schema + serde::Serialize>(msg: &M) -> Event {
    let payload = serde_json::to_value(msg).unwrap_or(serde_json::Value::Null);
    Event::new(M::schema_id(), payload)
}

/// The bridge actor — kameo bus subscriber, canvas topic publisher.
///
/// Subscribes to exactly the seven message types consumed by the ported
/// canvas slice actors and republishes each onto its topic. Holds the
/// [`ActorSystem`] handle it forwards through.
pub struct CanvasBridgeActor {
    /// The canvas system the bridge publishes into.
    system: std::sync::Arc<ActorSystem>,
}

/// Dependencies for spawning a [`CanvasBridgeActor`].
#[derive(Clone)]
pub struct CanvasBridgeDeps {
    /// Universal actor dependencies (bus subscription handle).
    pub deps: ActorDeps,
    /// The canvas system to forward into.
    pub system: std::sync::Arc<ActorSystem>,
}

impl Actor for CanvasBridgeActor {
    type Args = CanvasBridgeDeps;
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
/// canvas topic. Topic sends resolve even with zero subscribers (the log
/// entry lands unread), so a returned error means a broken system — warn
/// and continue, matching the bus's fire-and-forget posture.
macro_rules! forward {
    ($self:expr, $msg:expr, $topic:expr, $ty:ty) => {{
        let topic: Topic = $topic;
        let event = event_of(&$msg);
        if let Err(_unroutable) = $self.system.send($self.system.envelope_to_topic(event, topic)).await {
            tracing::warn!(
                schema = %<$ty as Schema>::schema_id().to_string(),
                "canvas topic send returned an unroutable envelope"
            );
        }
    }};
}

impl Message<ActorStarting> for CanvasBridgeActor {
    type Reply = ();

    async fn handle(&mut self, msg: ActorStarting, _ctx: &mut Context<Self, Self::Reply>) {
        forward!(self, msg, fabric_topic(), ActorStarting);
    }
}

impl Message<ActorStarted> for CanvasBridgeActor {
    type Reply = ();

    async fn handle(&mut self, msg: ActorStarted, _ctx: &mut Context<Self, Self::Reply>) {
        forward!(self, msg, fabric_topic(), ActorStarted);
    }
}

impl Message<ActorShutdownCompleted> for CanvasBridgeActor {
    type Reply = ();

    async fn handle(&mut self, msg: ActorShutdownCompleted, _ctx: &mut Context<Self, Self::Reply>) {
        forward!(self, msg, fabric_topic(), ActorShutdownCompleted);
    }
}

impl Message<BrowserBinaryVerified> for CanvasBridgeActor {
    type Reply = ();

    async fn handle(&mut self, msg: BrowserBinaryVerified, _ctx: &mut Context<Self, Self::Reply>) {
        forward!(self, msg, fabric_topic(), BrowserBinaryVerified);
    }
}

impl Message<DiscordStatusUpdate> for CanvasBridgeActor {
    type Reply = ();

    async fn handle(&mut self, msg: DiscordStatusUpdate, _ctx: &mut Context<Self, Self::Reply>) {
        forward!(self, msg, fabric_topic(), DiscordStatusUpdate);
    }
}

impl Message<DashboardNav> for CanvasBridgeActor {
    type Reply = ();

    async fn handle(&mut self, msg: DashboardNav, _ctx: &mut Context<Self, Self::Reply>) {
        forward!(self, msg, dashboard_topic(), DashboardNav);
    }
}

impl Message<SubmitQuakeBarCommand> for CanvasBridgeActor {
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
pub async fn spawn(services: &Services) -> ActorRef<CanvasBridgeActor> {
    let actor = CanvasBridgeActor::supervise(
        &services.root_supervisor,
        CanvasBridgeDeps {
            deps: ActorDeps {
                services: services.clone(),
            },
            system: services.canvas_system.clone(),
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
    use actor_runtime::types::SchemaId;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use actor_runtime::actor::MsgHandler;
    use actor_runtime::context::MsgCtx;
    use actor_runtime::registry::RegistryError;
    use actor_runtime::types::ActorPath;

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

    fn roundtrip<M: Schema + serde::Serialize + serde::de::DeserializeOwned>(msg: &M) -> M {
        serde_json::from_value(serde_json::to_value(msg).unwrap()).unwrap()
    }

    /// A probe service actor counting the fabric envelopes it receives.
    struct ProbeActor {
        hits: Arc<AtomicUsize>,
    }

    impl actor_runtime::actor::ServiceActor for ProbeActor {
        async fn start(
            _args: &serde_json::Value,
        ) -> Result<Self, actor_runtime::error_stack::Report<RegistryError>> {
            unreachable!("spawned via start_with; start is never called")
        }
    }

    impl MsgHandler<ActorStarted> for ProbeActor {
        async fn handle(&mut self, _msg: ActorStarted, _ctx: &mut MsgCtx<'_>) {
            self.hits.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// A bus publish lands on the canvas topic as a schema-tagged JSON
    /// envelope the probe's subscription receives.
    #[rstest::rstest]
    #[tokio::test]
    async fn bridge_translates_bus_publish_to_topic_envelope() {
        // Given a canvas system, a bridge forwarding the bus into it, and
        // a probe actor subscribed to the fabric topic.
        let services = Services::new_fake().await;
        spawn(&services).await;
        let hits = Arc::new(AtomicUsize::new(0));
        let system = services.canvas_system.clone();
        actor_runtime::builder::spawn_service_builder::<ProbeActor>(&system)
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
