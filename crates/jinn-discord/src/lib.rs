//! Discord bot frontend for jinn.
//!
//! Owns the poise gateway task and slash-command handlers that drive a running
//! jinn instance from Discord. The domain-layer pieces (config, thread-map DAO,
//! message splitter, bridge actor) live in [`jinn_domain::feat::discord`].
//!
//! See `.plans/discord/plan.md` for the full architecture.

pub mod commands;
pub mod feat;
pub mod gateway;
pub mod session_route;
pub mod spawn;

pub use gateway::{BotData, SpawnError};
pub use spawn::spawn_gateway;
