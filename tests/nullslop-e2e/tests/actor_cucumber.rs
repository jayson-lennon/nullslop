//! Actor-level cucumber test entry point.
//!
//! Runs all `.feature` files under `tests/features/actor/` using the [`ActorWorld`].

mod actor_world;

use actor_world::ActorWorld;
use cucumber::World;

#[tokio::main]
async fn main() {
    ActorWorld::run("tests/features/actor").await;
}
