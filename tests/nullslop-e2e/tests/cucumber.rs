//! Cucumber test entry point.
//!
//! Runs all `.feature` files under `tests/features/` using the [`TuiWorld`].

mod world;

use cucumber::World;
use world::TuiWorld;

#[tokio::main]
async fn main() {
    TuiWorld::run("tests/features/tui").await;
}
