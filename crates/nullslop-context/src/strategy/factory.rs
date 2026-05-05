//! Default factory for creating prompt assembly strategies.
//!
//! Maps [`PromptStrategyId`] values to concrete strategy instances.
//! Supports passthrough and sliding window strategies.

use error_stack::Report;

use crate::strategy::compaction::CompactionStrategy;
use crate::strategy::passthrough::PassthroughStrategy;
use crate::strategy::sliding_window::SlidingWindowStrategy;
use crate::strategy::token_budget::TokenBudgetStrategy;
use crate::strategy::token_estimator::CharRatioEstimator;
use crate::strategy::types::{
    PromptAssembly, PromptAssemblyError, StrategyFactory,
};
use nullslop_protocol::PromptStrategyId;

/// Default sliding window size used when no configuration is provided.
const DEFAULT_SLIDING_WINDOW_SIZE: usize = 50;

/// Default token budget used when no configuration is provided.
const DEFAULT_TOKEN_BUDGET: usize = 8192;

/// The default strategy factory.
///
/// Creates strategies by their [`PromptStrategyId`]:
/// - `passthrough` → [`PassthroughStrategy`]
/// - `sliding_window` → [`SlidingWindowStrategy`] with default window size
pub struct DefaultStrategyFactory;

impl StrategyFactory for DefaultStrategyFactory {
    fn create(&self, id: &PromptStrategyId) -> Result<Box<dyn PromptAssembly>, Report<PromptAssemblyError>> {
        match id.to_string().as_str() {
            "passthrough" => Ok(Box::new(PassthroughStrategy)),
            "sliding_window" => Ok(Box::new(SlidingWindowStrategy::new(DEFAULT_SLIDING_WINDOW_SIZE))),
            "token_budget" => Ok(Box::new(TokenBudgetStrategy::new(
                DEFAULT_TOKEN_BUDGET,
                Box::new(CharRatioEstimator),
            ))),
            "compaction" => Ok(Box::new(CompactionStrategy::new(
                DEFAULT_TOKEN_BUDGET,
                Box::new(CharRatioEstimator),
            ))),
            _ => Err(Report::new(PromptAssemblyError)
                .attach(format!("unknown strategy: {id}"))),
        }
    }

    fn name(&self) -> &'static str {
        "default_strategy_factory"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factory_creates_passthrough() {
        let factory = DefaultStrategyFactory;
        let strategy = factory.create(&PromptStrategyId::passthrough()).expect("create");
        assert_eq!(strategy.name(), "passthrough");
    }

    #[test]
    fn factory_creates_sliding_window() {
        let factory = DefaultStrategyFactory;
        let strategy = factory.create(&PromptStrategyId::sliding_window()).expect("create");
        assert_eq!(strategy.name(), "sliding_window");
    }

    #[test]
    fn factory_creates_token_budget() {
        let factory = DefaultStrategyFactory;
        let strategy = factory.create(&PromptStrategyId::token_budget()).expect("create");
        assert_eq!(strategy.name(), "token_budget");
    }

    #[test]
    fn factory_creates_compaction() {
        let factory = DefaultStrategyFactory;
        let strategy = factory.create(&PromptStrategyId::compaction()).expect("create");
        assert_eq!(strategy.name(), "compaction");
    }

    #[test]
    fn factory_rejects_unknown_strategy() {
        let factory = DefaultStrategyFactory;
        let result = factory.create(&PromptStrategyId::new("nonexistent"));
        assert!(result.is_err());
    }

    #[test]
    fn factory_name() {
        let factory = DefaultStrategyFactory;
        assert_eq!(factory.name(), "default_strategy_factory");
    }
}
