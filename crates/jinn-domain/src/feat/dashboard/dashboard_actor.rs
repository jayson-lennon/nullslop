//! The dashboard actor — owns `frontend.dashboard`.
//!
//! Aggregates two data sources into a single dashboard view:
//!
//! - **Generic actor lifecycle** — subscribes to the existing bus events
//!   [`ActorStarting`], [`ActorStarted`], and [`ActorShutdownCompleted`] to
//!   track every actor's `Starting`/`Running`/`Dead` phase.
//! - **Generic service status** — subscribes to [`ServiceStatusUpdate`]
//!   events published by whichever feature owns a service, applying the
//!   optional lifecycle, description, and status message to the named row.
//!
//! This actor owns `frontend.dashboard` exclusively. No other code writes to
//! it. Status sources are symmetric producers: they publish events, and this
//! actor is the single sink. It has no knowledge of any individual feature.

use kameo::actor::ActorRef;
use kameo::prelude::{Actor, Context, Message};

use crate::common::actor::protocol::event::{ActorShutdownCompleted, ActorStarted, ActorStarting};
use crate::common::actor_deps::ActorDeps;
use crate::common::state::State;
use crate::feat::dashboard::{ActorLifecycle, ServiceStatusUpdate};

/// The dashboard actor.
///
/// Subscribes to generic lifecycle events and [`ServiceStatusUpdate`] and
/// writes all updates into `frontend.dashboard`.
pub struct DashboardActor {
    state: State,
    cap: crate::common::tcaps::frontend::FrontendCap,
}

/// Dependencies for [`DashboardActor`].
#[derive(Clone)]
pub struct DashboardActorDeps {
    /// Universal actor dependencies (bus subscription handle).
    pub deps: ActorDeps,
    /// Shared application state — the dashboard sub-struct is written here.
    pub state: State,
    /// Capability to write `frontend.dashboard`.
    pub cap: crate::common::tcaps::frontend::FrontendCap,
}

impl Actor for DashboardActor {
    type Args = DashboardActorDeps;
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
            .subscribe(actor_ref.recipient::<ServiceStatusUpdate>())
            .await;

        Ok(Self {
            state: args.state,
            cap: args.cap,
        })
    }
}

impl Message<ActorStarting> for DashboardActor {
    type Reply = ();

    async fn handle(&mut self, msg: ActorStarting, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.with_dashboard(&self.cap, |ops| {
            ops.dashboard().mark_starting(&msg.name, msg.description);
        });
    }
}

impl Message<ActorStarted> for DashboardActor {
    type Reply = ();

    async fn handle(&mut self, msg: ActorStarted, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.with_dashboard(&self.cap, |ops| {
            ops.dashboard().mark_running(&msg.name, msg.description);
        });
    }
}

impl Message<ActorShutdownCompleted> for DashboardActor {
    type Reply = ();

    async fn handle(&mut self, msg: ActorShutdownCompleted, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.with_dashboard(&self.cap, |ops| {
            ops.dashboard().mark_dead(&msg.name, None);
        });
    }
}

impl Message<ServiceStatusUpdate> for DashboardActor {
    type Reply = ();

    async fn handle(&mut self, msg: ServiceStatusUpdate, _ctx: &mut Context<Self, Self::Reply>) {
        self.state.with_dashboard(&self.cap, |ops| {
            let dashboard = ops.dashboard();
            match msg.lifecycle {
                Some(lifecycle) => match lifecycle {
                    ActorLifecycle::Starting => {
                        dashboard.mark_starting(&msg.name, msg.description);
                    }
                    ActorLifecycle::Running => {
                        dashboard.mark_running(&msg.name, msg.description);
                    }
                    ActorLifecycle::Dead => {
                        dashboard.mark_dead(&msg.name, msg.description);
                    }
                },
                None => {}
            }
            if msg.status_message.is_some() {
                dashboard.set_status_message(&msg.name, msg.status_message);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable,
        clippy::indexing_slicing,
        reason = "test code"
    )]
    use super::*;
    use crate::common::app_state::AppState;
    use crate::common::bus::test_harness::TestHarness;
    use crate::feat::dashboard::ActorLifecycle;
    use kameo::actor::Spawn;

    fn dashboard_entry(
        state: &State,
        name: &str,
    ) -> Option<(ActorLifecycle, Option<String>, Option<String>)> {
        let g = state.read();
        g.frontend
            .dashboard
            .actors()
            .iter()
            .find(|e| e.name == name)
            .map(|e| (e.lifecycle, e.status_message.clone(), e.description.clone()))
    }

    async fn spawn_actor(harness: &TestHarness, state: State) -> ActorRef<DashboardActor> {
        let actor = DashboardActor::spawn(DashboardActorDeps {
            deps: harness.actor_deps().await,
            state,
            cap: crate::common::tcaps::mint::mint_frontend_cap(),
        });
        actor.wait_for_startup().await;
        actor
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn actor_starting_event_creates_entry_with_starting_lifecycle() {
        // Given a DashboardActor.
        let harness = TestHarness::new().await;
        let state = State::new(AppState::default());
        spawn_actor(&harness, state.clone()).await;

        // When publishing ActorStarting.
        harness
            .publish(ActorStarting {
                name: "llm".to_owned(),
                description: None,
            })
            .await;

        // Then the dashboard shows the actor as Starting.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let (lifecycle, _, _) = dashboard_entry(&state, "llm").expect("entry should exist");
        assert_eq!(lifecycle, ActorLifecycle::Starting);
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn actor_started_event_transitions_to_running() {
        // Given a DashboardActor.
        let harness = TestHarness::new().await;
        let state = State::new(AppState::default());
        spawn_actor(&harness, state.clone()).await;

        // When publishing ActorStarted.
        harness
            .publish(ActorStarted {
                name: "llm".to_owned(),
                description: None,
            })
            .await;

        // Then the dashboard shows the actor as Running.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let (lifecycle, _, _) = dashboard_entry(&state, "llm").expect("entry should exist");
        assert_eq!(lifecycle, ActorLifecycle::Running);
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn actor_shutdown_event_transitions_to_dead() {
        // Given a DashboardActor with a running actor entry.
        let harness = TestHarness::new().await;
        let state = State::new(AppState::default());
        spawn_actor(&harness, state.clone()).await;
        harness
            .publish(ActorStarted {
                name: "llm".to_owned(),
                description: None,
            })
            .await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // When publishing ActorShutdownCompleted.
        harness
            .publish(ActorShutdownCompleted {
                name: "llm".to_owned(),
            })
            .await;

        // Then the dashboard shows the actor as Dead.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let (lifecycle, _, _) = dashboard_entry(&state, "llm").expect("entry should exist");
        assert_eq!(lifecycle, ActorLifecycle::Dead);
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn service_status_update_with_lifecycle_creates_described_entry() {
        // Given a DashboardActor subscribed to ServiceStatusUpdate.
        let harness = TestHarness::new().await;
        let state = State::new(AppState::default());
        spawn_actor(&harness, state.clone()).await;

        // When publishing a lifecycle-carrying update for a new row.
        harness
            .publish(ServiceStatusUpdate {
                name: "some-service".to_owned(),
                description: Some("What this service does".to_owned()),
                lifecycle: Some(ActorLifecycle::Dead),
                status_message: Some("Error: bad config".to_owned()),
            })
            .await;

        // Then the entry is created with the supplied description and lifecycle.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let (lifecycle, message, description) =
            dashboard_entry(&state, "some-service").expect("entry should exist");
        assert_eq!(lifecycle, ActorLifecycle::Dead);
        // And the status message is applied after the lifecycle mark.
        assert_eq!(message.as_deref(), Some("Error: bad config"));
        // And the description is stored.
        assert_eq!(description.as_deref(), Some("What this service does"));
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn service_status_update_without_lifecycle_only_writes_status_message() {
        // Given a DashboardActor and an existing row.
        let harness = TestHarness::new().await;
        let state = State::new(AppState::default());
        spawn_actor(&harness, state.clone()).await;
        harness
            .publish(ActorStarted {
                name: "some-service".to_owned(),
                description: None,
            })
            .await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // When publishing a lifecycle-free status update for that row.
        harness
            .publish(ServiceStatusUpdate {
                name: "some-service".to_owned(),
                description: None,
                lifecycle: None,
                status_message: Some("Chrome 138".to_owned()),
            })
            .await;

        // Then only the status message changes.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let (lifecycle, message, description) =
            dashboard_entry(&state, "some-service").expect("entry should exist");
        assert_eq!(message.as_deref(), Some("Chrome 138"));
        // And the lifecycle is untouched (still owned by the lifecycle events).
        assert_eq!(lifecycle, ActorLifecycle::Running);
        // And the description is preserved.
        assert_eq!(description, None);
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn service_status_update_status_only_creates_missing_entry_as_starting() {
        // Given a DashboardActor with no row for the service yet.
        let harness = TestHarness::new().await;
        let state = State::new(AppState::default());
        spawn_actor(&harness, state.clone()).await;

        // When publishing a status-only update for a missing row.
        harness
            .publish(ServiceStatusUpdate {
                name: "some-service".to_owned(),
                description: None,
                lifecycle: None,
                status_message: Some("Connecting".to_owned()),
            })
            .await;

        // Then the entry is created as Starting (set_status_message semantics).
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let (lifecycle, message, _) =
            dashboard_entry(&state, "some-service").expect("entry should exist");
        assert_eq!(lifecycle, ActorLifecycle::Starting);
        assert_eq!(message.as_deref(), Some("Connecting"));
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn service_status_update_without_status_message_leaves_message_alone() {
        // Given a DashboardActor and a row that already has a status message.
        let harness = TestHarness::new().await;
        let state = State::new(AppState::default());
        spawn_actor(&harness, state.clone()).await;
        harness
            .publish(ServiceStatusUpdate {
                name: "some-service".to_owned(),
                description: None,
                lifecycle: Some(ActorLifecycle::Running),
                status_message: Some("Connected".to_owned()),
            })
            .await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // When publishing a lifecycle-only update (no status message).
        harness
            .publish(ServiceStatusUpdate {
                name: "some-service".to_owned(),
                description: None,
                lifecycle: Some(ActorLifecycle::Dead),
                status_message: None,
            })
            .await;

        // Then the lifecycle transitions to Dead.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let (lifecycle, message, _) =
            dashboard_entry(&state, "some-service").expect("entry should exist");
        assert_eq!(lifecycle, ActorLifecycle::Dead);
        // And the previous status message is preserved.
        assert_eq!(message.as_deref(), Some("Connected"));
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn service_status_update_without_any_fields_is_a_noop_for_existing_row() {
        // Given a DashboardActor and an existing row with fixed data.
        let harness = TestHarness::new().await;
        let state = State::new(AppState::default());
        spawn_actor(&harness, state.clone()).await;
        harness
            .publish(ActorStarted {
                name: "some-service".to_owned(),
                description: Some("Surfs the web".to_owned()),
            })
            .await;
        harness
            .publish(ServiceStatusUpdate {
                name: "some-service".to_owned(),
                description: None,
                lifecycle: None,
                status_message: Some("Chrome 138".to_owned()),
            })
            .await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // When publishing an entirely empty status update.
        harness
            .publish(ServiceStatusUpdate {
                name: "some-service".to_owned(),
                description: None,
                lifecycle: None,
                status_message: None,
            })
            .await;

        // Then nothing about the row changed.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let (lifecycle, message, description) =
            dashboard_entry(&state, "some-service").expect("entry should exist");
        assert_eq!(lifecycle, ActorLifecycle::Running);
        assert_eq!(message.as_deref(), Some("Chrome 138"));
        assert_eq!(description.as_deref(), Some("Surfs the web"));
    }
}
