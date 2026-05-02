//! [`FakeUiElement`] for testing registry and rendering behavior.
//!
//! Records render calls so tests can verify that the registry dispatches
//! rendering correctly. The call log remains accessible to the test after
//! the element is moved into a
//! [`UiRegistry`](crate::UiRegistry).
//!
//! # Usage
//!
//! ```ignore
//! let (element, calls) = FakeUiElement::new("chat-input");
//! registry.register(Box::new(element));
//! // ... render ...
//! assert_eq!(calls.borrow().len(), 1);
//! ```

use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

use ratatui::{Frame, layout::Rect};

use crate::element::UiElement;

/// Recorded render call data: the allocated area and a snapshot of the input buffer.
pub type RenderCall = (Rect, String);

/// Fake UI element that records render calls.
///
/// The call log remains accessible to the test even after the element
/// is moved into the registry, following the same pattern as
/// [`FakeCommandHandler`] in `nullslop-component-core`.
///
/// [`FakeCommandHandler`]: nullslop_component_core::fake::FakeCommandHandler
#[derive(Debug)]
pub struct FakeUiElement<S> {
    /// Element name used for lookup.
    name: String,
    /// Recorded render invocations.
    render_calls: Rc<RefCell<Vec<RenderCall>>>,
    /// Marker for the unused state type parameter.
    _phantom: PhantomData<S>,
}

impl<S> FakeUiElement<S> {
    /// Create a new fake element with the given name.
    ///
    /// Returns a tuple of `(element, call_log)`. The element should be
    /// registered with a [`UiRegistry`](crate::UiRegistry); the call log
    /// is kept by the test for assertions.
    pub fn new(name: &str) -> (Self, Rc<RefCell<Vec<RenderCall>>>) {
        let render_calls = Rc::new(RefCell::new(Vec::new()));
        let element = Self {
            name: name.to_owned(),
            render_calls: Rc::clone(&render_calls),
            _phantom: PhantomData,
        };
        (element, render_calls)
    }
}

impl<S: std::fmt::Debug + 'static> UiElement<S> for FakeUiElement<S> {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn render(&mut self, _frame: &mut Frame<'_>, area: Rect, _state: &S) {
        self.render_calls.borrow_mut().push((area, String::new()));
    }
}

#[cfg(test)]
mod tests {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    use super::*;

    /// Helper to render an element via a real ratatui frame.
    ///
    /// Uses `Terminal::draw()` to obtain a frame, which is the standard
    /// way to create a `Frame` in ratatui 0.30+.
    fn render_element(element: &mut dyn crate::UiElement<()>, area: Rect, state: ()) {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend should init");
        terminal
            .draw(|frame| {
                element.render(frame, area, &state);
            })
            .expect("draw should succeed");
    }

    #[test]
    fn name_returns_correct_value() {
        // Given a fake element.
        let (element, _calls): (FakeUiElement<()>, _) = FakeUiElement::new("chat-input");

        // When querying the name.
        let name = element.name();

        // Then it matches the constructor argument.
        assert_eq!(name, "chat-input");
    }

    #[test]
    fn records_render_calls() {
        // Given a fake element.
        let (mut element, calls): (FakeUiElement<()>, _) = FakeUiElement::new("test");
        let state = ();

        // When rendering with a specific area.
        let area = Rect::new(0, 0, 80, 3);
        render_element(&mut element, area, state);

        // Then the call was recorded.
        assert_eq!(calls.borrow().len(), 1);
        let (recorded_area, _) = calls.borrow()[0].clone();
        assert_eq!(recorded_area, area);
    }

    #[test]
    fn shared_call_log_after_move() {
        // Given a fake element whose call_log is cloned.
        let (element, calls): (FakeUiElement<()>, _) = FakeUiElement::new("test");
        let calls_clone = Rc::clone(&calls);

        // When moving the element (simulating registry registration).
        drop(element);

        // Then the call log is still accessible via the Rc.
        assert!(calls_clone.borrow().is_empty());
    }

    #[test]
    fn multiple_render_calls_accumulate() {
        // Given a fake element.
        let (mut element, calls): (FakeUiElement<()>, _) = FakeUiElement::new("test");
        let state = ();
        let area1 = Rect::new(0, 0, 40, 10);
        let area2 = Rect::new(0, 10, 40, 10);

        // When rendering the element twice.
        render_element(&mut element, area1, state);
        render_element(&mut element, area2, state);

        // Then both calls were recorded.
        assert_eq!(calls.borrow().len(), 2);
        assert_eq!(calls.borrow()[0].0, area1);
        assert_eq!(calls.borrow()[1].0, area2);
    }
}
