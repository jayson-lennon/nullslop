//! Bus-level cucumber test entry point.
//!
//! Runs all `.feature` files under `tests/features/` using the [`BusWorld`].

mod bus_world;

use bus_world::BusWorld;
use cucumber::World;

#[tokio::main]
async fn main() {
    BusWorld::run("tests/features/bus").await;
}
