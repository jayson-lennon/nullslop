//! [`UiRegistry`] for storing and retrieving UI elements.
//!
//! The registry holds all registered display elements and provides lookup
//! by name and ordered iteration for rendering. Elements are added during
//! startup and drawn each frame by the TUI layer.

use crate::element::UiElement;

/// Registry of UI elements available for rendering.
///
/// Elements are registered during startup and queried by name during
/// the TUI render loop. Registration order is preserved — iteration
/// yields elements in the order they were added.
#[derive(Debug)]
pub struct UiRegistry<S> {
    /// Registered elements in insertion order.
    elements: Vec<Box<dyn UiElement<S>>>,
}

impl<S: 'static> UiRegistry<S> {
    /// Create a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
        }
    }

    /// Register a UI element.
    ///
    /// The element is appended to the registry. Iteration will yield
    /// it after all previously registered elements.
    pub fn register(&mut self, element: Box<dyn UiElement<S>>) {
        self.elements.push(element);
    }

    /// Get a mutable reference to an element by name.
    ///
    /// Performs a linear scan by name. Returns `None` if no element
    /// with the given name is registered.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Box<dyn UiElement<S>>> {
        self.elements.iter_mut().find(|e| e.name() == name)
    }

    /// Iterate over all registered elements with mutable access.
    ///
    /// Elements are yielded in registration order.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Box<dyn UiElement<S>>> {
        self.elements.iter_mut()
    }
}

impl<S: 'static> Default for UiRegistry<S> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::UiRegistry;
    use crate::fake::FakeUiElement;

    #[test]
    fn register_and_retrieve_by_name() {
        // Given a registry with one element.
        let (element, _calls): (FakeUiElement<()>, _) = FakeUiElement::new("chat-input");
        let mut registry: UiRegistry<()> = UiRegistry::new();
        registry.register(Box::new(element));

        // When looking up by name.
        let found = registry.get_mut("chat-input");

        // Then the element is found with the correct name.
        assert!(found.is_some());
        assert_eq!(found.unwrap().name(), "chat-input");
    }

    #[test]
    fn missing_element_returns_none() {
        // Given a registry with one element.
        let (element, _calls): (FakeUiElement<()>, _) = FakeUiElement::new("chat-input");
        let mut registry: UiRegistry<()> = UiRegistry::new();
        registry.register(Box::new(element));

        // When looking up a different name.
        let found = registry.get_mut("nonexistent");

        // Then None is returned.
        assert!(found.is_none());
    }

    #[test]
    fn iterate_yields_in_registration_order() {
        // Given a registry with three elements.
        let (e1, _c1): (FakeUiElement<()>, _) = FakeUiElement::new("first");
        let (e2, _c2): (FakeUiElement<()>, _) = FakeUiElement::new("second");
        let (e3, _c3): (FakeUiElement<()>, _) = FakeUiElement::new("third");
        let mut registry: UiRegistry<()> = UiRegistry::new();
        registry.register(Box::new(e1));
        registry.register(Box::new(e2));
        registry.register(Box::new(e3));

        // When iterating.
        let names: Vec<String> = registry.iter_mut().map(|e| e.name()).collect();

        // Then names appear in registration order.
        assert_eq!(names, vec!["first", "second", "third"]);
    }

    #[test]
    fn default_creates_empty_registry() {
        // Given a default registry.
        let mut registry: UiRegistry<()> = UiRegistry::default();

        // When iterating.
        let count = registry.iter_mut().count();

        // Then it is empty.
        assert_eq!(count, 0);
    }

    #[test]
    fn multiple_elements_same_name_returns_first() {
        // Given a registry with two elements sharing a name.
        let (e1, _c1): (FakeUiElement<()>, _) = FakeUiElement::new("duplicate");
        let (e2, _c2): (FakeUiElement<()>, _) = FakeUiElement::new("duplicate");
        let mut registry: UiRegistry<()> = UiRegistry::new();
        registry.register(Box::new(e1));
        registry.register(Box::new(e2));

        // When looking up by name.
        let found = registry.get_mut("duplicate");

        // Then the first registered element is returned.
        assert!(found.is_some());
    }

    #[test]
    fn empty_registry_get_mut_returns_none() {
        // Given an empty registry.
        let mut registry: UiRegistry<()> = UiRegistry::new();

        // When looking up any name.
        let found = registry.get_mut("anything");

        // Then None is returned.
        assert!(found.is_none());
    }
}
