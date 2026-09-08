//! Generic actor lifecycle phase, applicable to every actor in the system.
//!
//! A pure value type: the dashboard folds bus lifecycle events
//! (`ActorStarting`, `ActorStarted`, `ActorShutdownCompleted`) into it,
//! and any consumer of actor status can compare against it without
//! depending on `jinn-domain`.

/// The lifecycle phase of an actor.
///
/// Driven by the existing bus events: `ActorStarting`, `ActorStarted`, and
/// `ActorShutdownCompleted`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorLifecycle {
    /// The actor is currently starting up.
    Starting,
    /// The actor has finished starting and is ready.
    Running,
    /// The actor has shut down (crashed or intentional).
    Dead,
}
