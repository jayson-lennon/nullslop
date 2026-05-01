//! LLM service abstraction — streaming chat completions.
//!
//! Defines the [`LlmService`] trait for streaming LLM responses and
//! [`LlmServiceFactory`] for creating per-call service instances.
//! Includes an `OpenRouter` implementation and a fake for testing.

mod convert;
mod fake;
mod openrouter;
mod sample;
mod service;
mod service_wrapper;

pub use convert::llm_messages_to_chat_messages;
pub use fake::FakeLlmServiceFactory;
pub use openrouter::{ApiKey, OpenRouterLlmServiceFactory};
pub use sample::SampleLlmServiceFactory;
pub use service::{ChatStream, LlmService, LlmServiceError, LlmServiceFactory};
pub use service_wrapper::LlmServiceFactoryService;
