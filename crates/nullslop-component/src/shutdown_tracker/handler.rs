//! Reacts to actor lifecycle events during shutdown.
//!
//! Keeps track of which actors are running, notices when shutdown is requested,
//! waits for each actor to finish, and signals the application to proceed once
//! all actors have completed.

use crate::AppState;
use npr::actor::ProceedWithShutdown;
use npr::actor::{ActorShutdownCompleted, ActorStarted, ActorStarting};
use nullslop_component_core::{HandlerContext, define_handler};
use nullslop_protocol as npr;
use nullslop_services::Services;

define_handler! {
    pub(crate) struct ShutdownTrackerHandler;

    commands {}

    events {
        ActorStarting: on_actor_starting,
        ActorStarted: on_actor_started,
        ActorShutdownCompleted: on_actor_shutdown_completed,
    }
}

impl ShutdownTrackerHandler {
    /// Tracks a new actor for shutdown monitoring.
    fn on_actor_starting(evt: &ActorStarting, ctx: &mut HandlerContext<'_, AppState, Services>) {
        ctx.state.shutdown_tracker.track(&evt.name);
        tracing::info!(name = %evt.name, "actor starting");
    }

    /// Logs that an actor has finished starting.
    fn on_actor_started(evt: &ActorStarted, _ctx: &mut HandlerContext<'_, AppState, Services>) {
        tracing::info!(name = %evt.name, "actor started");
    }

    /// Marks an actor as shut down and signals completion when all actors are done.
    fn on_actor_shutdown_completed(
        evt: &ActorShutdownCompleted,
        ctx: &mut HandlerContext<'_, AppState, Services>,
    ) {
        let was_tracked = ctx.state.shutdown_tracker.complete(&evt.name);
        if was_tracked {
            tracing::info!(name = %evt.name, "actor shutdown completed");
        }
        if ctx.state.shutdown_tracker.is_complete() {
            ctx.out.submit_command(npr::Command::ProceedWithShutdown {
                payload: ProceedWithShutdown {
                    completed: vec![evt.name.clone()],
                    timed_out: vec![],
                },
            });
        }
    }
}
