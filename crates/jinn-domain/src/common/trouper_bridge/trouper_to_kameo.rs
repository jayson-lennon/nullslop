//! Trouper → kameo bridge actor.
//!
//! The reverse half of the trouper bridge: a trouper `ServiceActor`
//! subscribed to the route topics, republishing every message it
//! receives onto the kameo bus, where the original (pre-port) consumers
//! still listen. See the parent [`crate::common::trouper_bridge`]
//! module for the two-fabric picture, the route tables, and the loop
//! guard.
//!
//! The actor must live on trouper — topic subscription is a trouper
//! `ActorPath` concept — so this is a [`ServiceActor`] whose kameo
//! [`BusService`] handle rides in through the builder's `start_with`
//! override (handles cannot ride JSON start args).

use trouper::actor::ServiceActor;
use trouper::registry::RegistryError;
use trouper::system::ActorSystem;
use trouper::types::ActorPath;

use crate::common::services::bus_service::BusService;

/// The reverse bridge actor — trouper topic subscriber, kameo bus
/// publisher.
///
/// One [`trouper::actor::MsgHandler`] impl per reverse route (generated
/// by [`reverse_routes!`]); each handler republishes the typed message
/// onto the kameo bus. Holds the bus handle it republishes through —
/// unread in the shipped zero-route configuration (no handlers, no
/// reads), which is expected until the first reverse route lands.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "zero routes: no handler reads the bus yet")
)]
pub struct TrouperToKameoBridgeActor {
    /// The kameo bus the bridge republishes onto.
    bus: BusService,
}

impl ServiceActor for TrouperToKameoBridgeActor {
    async fn start(
        _args: &serde_json::Value,
    ) -> Result<Self, trouper::error_stack::Report<RegistryError>> {
        Err(trouper::error_stack::IntoReport::into_report(RegistryError::InvalidSpec)
            .attach("TrouperToKameoBridgeActor is spawned via start_with; start requires the bus handle"))
    }
}

/// Declares the reverse bridge routes: `(MessageType, topic)` pairs
/// flowing trouper → kameo.
///
/// Expands to, per route, the actor's typed handler (republish onto the
/// kameo bus), the builder's `.handles::<M>()` registration (schema +
/// decode), and the topic subscription; plus the spawn helper and the
/// schema-id list the loop guard reads.
///
/// Registration direction is a human choice: a type listed here AND in
/// the forward bridge would loop forever, which
/// [`crate::common::trouper_bridge::debug_assert_no_fabric_loops`]
/// rejects in debug builds.
///
/// Ships with zero routes — the mechanism exists; the first
/// trouper-emitting slice registers the first route here.
macro_rules! reverse_routes {
    ([$(($ty:ty, $topic:expr)),* $(,)?]) => {
        $(
            impl trouper::actor::MsgHandler<$ty> for TrouperToKameoBridgeActor {
                async fn handle(
                    &mut self,
                    msg: $ty,
                    _ctx: &mut trouper::context::MsgCtx<'_>,
                ) {
                    self.bus.publish(msg).await;
                }
            }
        )*

        pub fn spawn_trouper_to_kameo(
            system: &std::sync::Arc<ActorSystem>,
            bus: BusService,
        ) -> ActorPath {
            let path = trouper::builder::spawn_service_builder::<TrouperToKameoBridgeActor>(system)
                .at(ActorPath::new("trouper-to-kameo"))
                .start_with(move || Box::pin(async move { Ok(TrouperToKameoBridgeActor { bus }) }))
                $(.handles::<$ty>())*
                .start();
            $(
                system
                    .subscribe(&path, &$topic, None)
                    .expect("trouper-to-kameo bridge subscribes to a route topic");
            )*
            $crate::common::trouper_bridge::debug_assert_no_fabric_loops();
            path
        }

        /// The reverse route table's schema ids — the loop guard's
        /// input. Debug builds read it through the spawn helper's
        /// [`debug_assert`]; tests read it to simulate misuse. Only
        /// release non-test builds see nothing, so the dead-code
        /// allowance is scoped to exactly that combination.
        #[cfg_attr(
            all(not(debug_assertions), not(test)),
            expect(dead_code, reason = "loop guard is debug-only; tests consume this directly")
        )]
        pub(crate) fn reverse_schema_ids() -> Vec<trouper::types::SchemaId> {
            vec![$(<$ty as trouper::schema::Schema>::schema_id()),*]
        }
    };
}

reverse_routes!([]);

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
    use std::time::Duration;

    use trouper::envelope::Event;
    use trouper::schema::{FieldTy, Schema, SchemaKind};

    use crate::common::trouper_bridge::impl_schema;
    use trouper::types::Topic;

    use crate::common::actor::protocol::event::ActorStarting;
    use crate::common::bus::test_harness::{TestHarness, await_recorded};

    /// Test-only topic carrying the probe route.
    const PROBE_TOPIC: &str = "jinn.test.reverse-bridge";

    /// Another topic with no reverse route — the negative case.
    const UNROUTED_TOPIC: &str = "jinn.test.reverse-bridge-unrouted";

    fn probe_topic() -> Topic {
        Topic::new(PROBE_TOPIC)
    }

    fn unrouted_topic() -> Topic {
        Topic::new(UNROUTED_TOPIC)
    }

    /// Test-only probe message — the stand-in for the first real
    /// trouper-emitting slice's event.
    #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    struct ProbeEvent {
        note: String,
    }

    impl crate::common::bus::BusMessage for ProbeEvent {}

    impl_schema!(ProbeEvent, "ProbeEvent", SchemaKind::Event,
        description: "Test-only probe for the reverse bridge.",
        fields: ["note" => FieldTy::Str]);

    // The test route set: exercises the same macro the production route
    // table uses, with one live route.
    reverse_routes!([(ProbeEvent, probe_topic())]);

    fn probe_envelope() -> Event {
        Event::new(
            ProbeEvent::schema_id(),
            serde_json::json!({ "note": "hello" }),
        )
    }

    /// A ProbeEvent published on its routed trouper topic arrives at a
    /// kameo subscriber, typed.
    #[rstest::rstest]
    #[tokio::test]
    async fn reverse_route_forwards_topic_message_to_kameo() {
        // Given the reverse bridge routed for ProbeEvent and a kameo
        // recorder registered for it.
        let harness = TestHarness::new().await;
        let services = harness.services().await;
        let system = services.trouper_system.clone();
        spawn_trouper_to_kameo(&system, services.bus.clone());
        let recorder = harness.spawn_recorder::<ProbeEvent>().await;

        // When publishing a ProbeEvent envelope on its routed topic
        // immediately after the spawn helper returns.
        system
            .send(system.envelope_to_topic(probe_envelope(), probe_topic()))
            .await
            .expect("topic send resolves");

        // Then the kameo recorder receives exactly the typed message —
        // subscribe is the readiness point, so nothing is missed.
        let msgs = await_recorded(&recorder, 1, Duration::from_secs(2)).await;
        assert_eq!(
            msgs,
            vec![ProbeEvent {
                note: "hello".to_owned()
            }]
        );
    }

    /// A message published on a topic without a reverse route never
    /// reaches kameo.
    #[rstest::rstest]
    #[tokio::test]
    async fn reverse_bridge_ignores_unregistered_topics() {
        // Given the reverse bridge routed for ProbeEvent and a kameo
        // recorder registered for it.
        let harness = TestHarness::new().await;
        let services = harness.services().await;
        let system = services.trouper_system.clone();
        spawn_trouper_to_kameo(&system, services.bus.clone());
        let recorder = harness.spawn_recorder::<ProbeEvent>().await;

        // When publishing a ProbeEvent envelope on an unrouted topic.
        system
            .send(system.envelope_to_topic(probe_envelope(), unrouted_topic()))
            .await
            .expect("topic send resolves");

        // Then nothing reaches the kameo recorder within the wait window.
        let msgs = await_recorded(&recorder, 1, Duration::from_millis(400)).await;
        assert!(
            msgs.is_empty(),
            "unrouted topic leaked into kameo: {msgs:?}"
        );
    }

    /// The shipped zero-route configuration spawns the bridge inert: a
    /// live actor at its well-known path, subscribed to nothing.
    #[rstest::rstest]
    #[tokio::test]
    async fn reverse_bridge_spawns_inert_with_zero_routes() {
        // Given the shipped zero-route configuration.
        let services = crate::Services::new_fake().await;
        let system = services.trouper_system.clone();

        // When spawning the reverse bridge via the module-level helper.
        let path = super::spawn_trouper_to_kameo(&system, services.bus.clone());

        // Then the bridge is live at its well-known path.
        assert_eq!(path.to_string(), "trouper-to-kameo");
    }

    /// Registering the same message type in both direction tables is
    /// rejected at debug time — kameo→trouper→kameo would loop forever.
    #[rstest::rstest]
    #[test]
    #[should_panic(expected = "registered in both bridge directions")]
    fn dual_direction_registration_panics_in_debug() {
        // Given the reverse route table extended with a type the forward
        // table also carries (`ActorStarting` is forward-registered).
        let mut reverse = reverse_schema_ids();
        reverse.push(<ActorStarting as Schema>::schema_id());

        // When the loop guard compares the mutated table against the
        // forward table.
        // Then it rejects the overlap (debug builds).
        crate::common::trouper_bridge::assert_tables_are_disjoint(
            &crate::common::trouper_bridge::forward_schema_ids(),
            &reverse,
        );
    }
}
