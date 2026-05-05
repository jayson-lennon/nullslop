//! Context domain: prompt assembly commands, events, and strategy identification.
//!
//! The prompt assembly system converts raw chat history into LLM-ready
//! message arrays using pluggable strategies. This module defines the
//! wire types for the assembly request/response cycle and strategy switching.

mod command;
mod event;
pub mod strategy_id;

pub use command::{AssemblePrompt, RestoreStrategyState, SwitchPromptStrategy};
pub use event::{PromptAssembled, PromptStrategySwitched, StrategyStateUpdated};
pub use strategy_id::PromptStrategyId;
