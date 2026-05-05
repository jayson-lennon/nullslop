//! Prompt assembly — strategies for building LLM-ready prompts from chat history.
//!
//! This crate defines the [`PromptAssembly`] trait and supporting types for
//! assembling conversation context into `LlmMessage` arrays suitable for
//! sending to LLM providers. Each strategy (passthrough, sliding window,
//! token budget, compaction) implements this trait and can be switched
//! at runtime per session.

mod strategy;

pub use nullslop_protocol::PromptStrategyId;
pub use strategy::compaction::CompactionStrategy;
pub use strategy::compaction_data::CompactionSessionData;
pub use strategy::factory::DefaultStrategyFactory;
pub use strategy::passthrough::PassthroughStrategy;
pub use strategy::sliding_window::SlidingWindowStrategy;
pub use strategy::token_budget::TokenBudgetStrategy;
pub use strategy::token_estimator::{CharRatioEstimator, TokenEstimator};
pub use strategy::types::{
    AssembledPrompt, AssemblyContext, PromptAssembly, PromptAssemblyError, StrategyFactory,
    StrategySessionData,
};
