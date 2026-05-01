//! Event routing infrastructure.

/// The type name used for event routing.
///
/// Used as `HashMap` keys, `Vec` elements, and function params in the
/// routing/subscription system. Backed by `String` so both static literals
/// and dynamically-constructed names work without restriction.
pub type EventTypeName = String;

/// Marker trait for event payloads that actors can subscribe to.
///
/// Each implementation provides a [`TYPE_NAME`](EventMsg::TYPE_NAME)
/// constant used for string-based routing (in-memory and process host).
pub trait EventMsg: Send + Sync + 'static {
    /// The subscription-relevant type name for event routing.
    const TYPE_NAME: &'static str;
}
