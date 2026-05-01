//! Actor command routing infrastructure.

/// The name used for command routing.
///
/// Used as `HashMap` keys, `Vec` elements, and function params in the
/// routing/subscription system. Backed by `String` so both static literals
/// and dynamically-constructed names work without restriction.
pub type CommandName = String;

/// Marker trait for actor commands that provides compile-time-checked names.
///
/// Each implementation provides a [`NAME`](CommandMsg::NAME) constant
/// used for command routing.
pub trait CommandMsg: Send + Sync + 'static {
    /// The command name used for routing.
    const NAME: &'static str;
}
