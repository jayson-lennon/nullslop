//! Service wrapper for LLM service factory.
//!
//! Wraps an [`LlmServiceFactory`] in a shared, swappable container. All clones
//! of [`LlmServiceFactoryService`] see the same factory — calling [`swap`](LlmServiceFactoryService::swap)
//! on one clone updates every clone. This enables runtime provider switching
//! without replacing the service wrapper itself.

use std::sync::Arc;

use error_stack::Report;
use parking_lot::RwLock;

use crate::service::{LlmService, LlmServiceError, LlmServiceFactory};

/// Swappable service wrapper for the LLM service factory.
///
/// Wraps `Arc<dyn LlmServiceFactory>` in a `parking_lot::RwLock` so that all
/// clones share the same underlying factory. Calling [`swap`](Self::swap)
/// replaces the factory for every clone.
///
/// Follows the project's service wrapper pattern.
#[derive(Debug, Clone)]
pub struct LlmServiceFactoryService {
    /// The wrapped factory implementation, protected by an [`RwLock`] for swapping.
    inner: Arc<RwLock<Arc<dyn LlmServiceFactory>>>,
}

impl LlmServiceFactoryService {
    /// Create a new service wrapper.
    #[must_use]
    pub fn new(factory: Arc<dyn LlmServiceFactory>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(factory)),
        }
    }

    /// Create a new LLM service instance via the current factory.
    ///
    /// # Errors
    ///
    /// Returns an error if the factory fails to create a service.
    pub fn create(&self) -> Result<Box<dyn LlmService>, Report<LlmServiceError>> {
        let guard = self.inner.read();
        guard.create()
    }

    /// Returns the current factory name.
    #[must_use]
    pub fn name(&self) -> String {
        let guard = self.inner.read();
        guard.name().to_owned()
    }

    /// Swaps the underlying factory for all clones of this service.
    ///
    /// The new factory takes effect immediately for subsequent [`create`](Self::create)
    /// calls. In-flight streams are unaffected — they use service instances already
    /// created by the previous factory.
    pub fn swap(&self, factory: Arc<dyn LlmServiceFactory>) {
        let mut guard = self.inner.write();
        *guard = factory;
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
        let factory = FakeLlmServiceFactory::new(vec!["token".to_owned()]);
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

    #[test]
    fn swap_updates_factory_for_all_clones() {
        // Given two clones of the same service wrapper.
        let factory_a = FakeLlmServiceFactory::new(vec![]);
        let service = LlmServiceFactoryService::new(Arc::new(factory_a));
        let clone = service.clone();

        assert_eq!(service.name(), "FakeLlm");

        // When swapping the factory on one clone.
        let factory_b = crate::sample::SampleLlmServiceFactory;
        clone.swap(Arc::new(factory_b));

        // Then both clones see the new factory.
        assert_eq!(service.name(), "Sample");
        assert_eq!(clone.name(), "Sample");
    }
}
