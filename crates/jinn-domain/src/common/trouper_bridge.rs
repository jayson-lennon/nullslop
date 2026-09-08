//! Kameo → trouper bridge.
//!
//! Jinn's kameo message bus and the trouper `ActorSystem` are two
//! separate fabrics. The slice actors that have been ported to
//! trouper (dashboard, quake-bar) can no longer subscribe to kameo
//! bus messages directly, so this module is the one translation seam:
//! [`KameoToTrouperBridgeActor`] subscribes to exactly the messages the
//! ported slices consume and republishes each one onto its trouper
//! topic, where the ported actors' topic subscriptions pick it up.
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

pub mod kameo_to_trouper;
pub mod trouper_to_kameo;

pub use kameo_to_trouper::{
    KameoToTrouperBridgeActor, KameoToTrouperBridgeDeps, spawn_kameo_to_trouper,
};
pub use trouper_to_kameo::{TrouperToKameoBridgeActor, spawn_trouper_to_kameo};

use trouper::envelope::Event;
use trouper::schema::{FieldTy, Schema, SchemaKind};
use trouper::types::{SchemaId, Topic};

use crate::common::actor::protocol::event::{ActorShutdownCompleted, ActorStarted, ActorStarting};
use crate::feat::browser_binary_scan::BrowserBinaryVerified;
use crate::feat::dashboard::nav::DashboardNav;
use crate::feat::discord::DiscordStatusUpdate;
use crate::feat::quake_bar::command::SubmitQuakeBarCommand;

/// Trouper topic names the bridge publishes onto.
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
/// `name` mirrors the Rust type name so trouper exports read the same on
/// both sides of the bridge; all crossing schemas are version 1.
macro_rules! impl_schema {
    ($ty:ty, $name:literal, $kind:expr, description: $desc:literal, fields: [$($field:literal => $fty:expr),* $(,)?]) => {
        impl ::trouper::schema::Schema for $ty {
            fn schema_def() -> ::trouper::schema::SchemaDef {
                ::trouper::schema::SchemaDef {
                    name: $name.to_owned(),
                    version: 1,
                    kind: $kind,
                    fields: vec![$(::trouper::schema::FieldDef::required($field, $fty)),*],
                    description: Some($desc.to_owned()),
                }
            }
        }
    };
}
// Re-exported for child modules' test route tables (the production
// route tables live in this module and use the macro textually).
#[cfg(test)]
pub(crate) use impl_schema;

/// The forward route table's schema ids — the messages registered
/// kameo → trouper.
///
/// Kept as a literal list (not derived from the actor's `Message`
/// impls) so the loop guard compares what a maintainer actually
/// registered, not what the compiler inferred.
pub(crate) fn forward_schema_ids() -> Vec<SchemaId> {
    vec![
        <SubmitQuakeBarCommand as Schema>::schema_id(),
        <DashboardNav as Schema>::schema_id(),
        <ActorStarting as Schema>::schema_id(),
        <ActorStarted as Schema>::schema_id(),
        <ActorShutdownCompleted as Schema>::schema_id(),
        <BrowserBinaryVerified as Schema>::schema_id(),
        <DiscordStatusUpdate as Schema>::schema_id(),
    ]
}

/// Rejects a message type registered in both bridge directions.
///
/// A type registered kameo → trouper AND trouper → kameo loops forever
/// (each bridge republishes what the other forwarded). Registration
/// direction is a human choice made in the route tables, so this is the
/// one way the mechanism can be misused — and it is checked whenever a
/// bridge spawns in a debug build.
///
/// # Panics
///
/// Panics in debug builds when the two id lists intersect.
pub(crate) fn assert_tables_are_disjoint(forward: &[SchemaId], reverse: &[SchemaId]) {
    let overlap: Vec<&SchemaId> = forward.iter().filter(|f| reverse.contains(f)).collect();
    debug_assert!(
        overlap.is_empty(),
        "bridge route tables are not disjoint: {overlap:?} is registered in both bridge \
         directions; remove it from one table or messages will loop between the fabrics"
    );
}

/// Checks the shipped route tables against each other. Called from both
/// spawn helpers.
pub(crate) fn debug_assert_no_fabric_loops() {
    assert_tables_are_disjoint(
        &forward_schema_ids(),
        &trouper_to_kameo::reverse_schema_ids(),
    );
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

/// Builds a trouper [`Event`] from a crossing message.
///
/// Serialization cannot fail for these types (plain structs/enums), so a
/// failure degrades to a null payload rather than a panic in an actor
/// handler.
pub(crate) fn event_of<M>(msg: &M) -> Event
where
    M: ::trouper::schema::Schema + serde::Serialize,
{
    let payload = serde_json::to_value(msg).unwrap_or(serde_json::Value::Null);
    Event::new(M::schema_id(), payload)
}

/// Forwards one message: JSON-serialize under its schema and send onto a
/// trouper topic. Topic sends resolve even with zero subscribers (the log
/// entry lands unread), so a returned error means a broken system — warn
/// and continue, matching the bus's fire-and-forget posture.
macro_rules! forward {
    ($self:expr, $msg:expr, $topic:expr, $ty:ty) => {{
        let topic: ::trouper::types::Topic = $topic;
        let event = $crate::common::trouper_bridge::event_of(&$msg);
        if let Err(_unroutable) =
            $self.system.send($self.system.envelope_to_topic(event, topic)).await
        {
            tracing::warn!(
                schema = %<$ty as ::trouper::schema::Schema>::schema_id().to_string(),
                "trouper topic send returned an unroutable envelope"
            );
        }
    }};
}
pub(crate) use forward;
