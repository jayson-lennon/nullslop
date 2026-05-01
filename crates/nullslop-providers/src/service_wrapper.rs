//! Service wrapper for LLM service factory.

use std::sync::Arc;

use error_stack::Report;

use crate::service::{LlmService, LlmServiceError, LlmServiceFactory};

/// Service wrapper for the LLM service factory.
///
/// Wraps `Arc<dyn LlmServiceFactory>` for shared ownership.
/// Follows the project's service wrapper pattern.
#[derive(Debug, Clone)]
pub struct LlmServiceFactoryService {
    inner: Arc<dyn LlmServiceFactory>,
}

impl LlmServiceFactoryService {
    /// Create a new service wrapper.
    #[must_use]
    pub fn new(factory: Arc<dyn LlmServiceFactory>) -> Self {
        Self { inner: factory }
    }

    /// Create a new LLM service instance via the factory.
    ///
    /// # Errors
    ///
    /// Returns an error if the factory fails to create a service.
    pub fn create(&self) -> Result<Box<dyn LlmService>, Report<LlmServiceError>> {
        self.inner.create()
    }

    /// Returns the factory name.
    #[must_use]
    pub fn name(&self) -> &'static str {
        self.inner.name()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::fake::FakeLlmServiceFactory;
    use crate::service_wrapper::LlmServiceFactoryService;

    #[test]
    fn service_wrapper_delegates_create() {
        // Given a service wrapper around a fake factory.
        let factory = FakeLlmServiceFactory::new(vec!["token".to_string()]);
        let service = LlmServiceFactoryService::new(Arc::new(factory));

        // When creating a service instance.
        let result = service.create();

        // Then it succeeds.
        assert!(result.is_ok());
    }

    #[test]
    fn service_wrapper_delegates_name() {
        // Given a service wrapper around a fake factory.
        let factory = FakeLlmServiceFactory::new(vec![]);
        let service = LlmServiceFactoryService::new(Arc::new(factory));

        // When asking for the name.
        // Then it returns the factory's name.
        assert_eq!(service.name(), "FakeLlm");
    }

    #[test]
    fn service_wrapper_is_cloneable() {
        // Given a service wrapper.
        let factory = FakeLlmServiceFactory::new(vec![]);
        let service = LlmServiceFactoryService::new(Arc::new(factory));

        // When cloning.
        let cloned = service.clone();

        // Then both point to the same factory.
        assert_eq!(service.name(), cloned.name());
    }
}
