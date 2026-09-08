//! Foundational, domain-agnostic value types shared across the jinn workspace.
//!
//! Residents here are pure value types (newtypes over primitives) with no
//! dependency on domain logic, actors, or app state. They exist so that leaf
//! crates can reference a shared type without depending on `jinn-domain`.
//!
//! Types are added as-needed. This is not a dumping ground: only types that are
//! both foundational and domain-agnostic belong here.

pub mod actor_lifecycle;
pub mod session_id;

pub use actor_lifecycle::ActorLifecycle;
pub use session_id::SessionId;
