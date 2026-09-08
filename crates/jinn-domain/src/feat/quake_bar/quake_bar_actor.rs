//! Quake bar actor — the sole owner/writer of the command log.
//!
//! Subscribes to [`SubmitQuakeBarCommand`] and appends the submitted
//! line to the slice's cell
//! ([`QuakeBarState::log`](super::state::QuakeBarState::log)). The cell
//! handle arrives in the actor's deps — a clone of the one handle
//! minted at activation (the intent-handler input hook holds another
//! clone for the input field only). Keeping this actor as the only
//! writer of the log lets future quake-specific debug commands and
//! event subscriptions funnel through one mutator.

use kameo::prelude::{Actor, ActorRef, Context, Message};

use jinn_slices::TypedCell;

use crate::common::actor_deps::ActorDeps;
use crate::feat::quake_bar::command::SubmitQuakeBarCommand;
use crate::feat::quake_bar::state::QuakeBarState;

/// Owns the quake bar command log.
///
/// The single subscriber to [`SubmitQuakeBarCommand`]; the only writer
/// of the log field of the slice cell.
pub struct QuakeBarActor {
    /// The slice cell (the one write handle, shared by clone).
    cell: TypedCell<QuakeBarState>,
}

/// Dependencies for spawning a [`QuakeBarActor`].
#[derive(Clone)]
pub struct QuakeBarActorDeps {
    /// Universal actor dependencies (bus, services, etc.).
    pub deps: ActorDeps,
    /// The quake bar's slice cell — the handle minted at activation.
    pub cell: TypedCell<QuakeBarState>,
}

impl Actor for QuakeBarActor {
    type Args = QuakeBarActorDeps;
    type Error = kameo::error::Infallible;

    async fn on_start(args: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        args.deps
            .subscribe(actor_ref.recipient::<SubmitQuakeBarCommand>())
            .await;
        Ok(Self { cell: args.cell })
    }
}

impl Message<SubmitQuakeBarCommand> for QuakeBarActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SubmitQuakeBarCommand,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.apply_submit(msg);
    }
}

impl QuakeBarActor {
    /// Appends the submitted text to the command log.
    fn apply_submit(&self, msg: SubmitQuakeBarCommand) {
        self.cell.update(|s| s.log.push(msg.text));
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

    use crate::feat::quake_bar::command::SubmitQuakeBarCommand;
    use crate::feat::quake_bar::state::QuakeBarState;
    use crate::feat::quake_bar::state::quake_bar_slot;
    use jinn_slices::Slices;

    use super::QuakeBarActor;

    fn create_actor() -> (QuakeBarActor, jinn_slices::TypedCell<QuakeBarState>) {
        let slices = Slices::new();
        let cell = slices
            .register(quake_bar_slot(), QuakeBarState::default())
            .expect("fresh registry");
        (QuakeBarActor { cell: cell.clone() }, cell)
    }

    #[rstest::rstest]
    #[test]
    fn submit_command_appends_text_to_log() {
        // Given a quake bar actor over its slice cell.
        let (actor, cell) = create_actor();

        // When applying a SubmitQuakeBarCommand.
        actor.apply_submit(SubmitQuakeBarCommand {
            text: "hello".to_owned(),
        });

        // Then the text appears in the command log.
        let guard = cell.read();
        assert_eq!(guard.log.len(), 1);
        assert_eq!(
            guard.log.visible_lines(5),
            &["hello".to_owned()]
        );
    }
}
