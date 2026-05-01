//! Actor name newtype for source filtering.

use std::ops::Deref;

use derive_more::Display;

/// The name of an actor, used for source filtering.
///
/// When an actor sends a command or event, its name is attached as the
/// source so the actor host can avoid routing it back to the originator.
///
/// Implements [`Deref<Target = str>`] so it can be used directly in string
/// comparisons and formatting.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display)]
pub struct ActorName(String);

impl ActorName {
    /// Creates a new actor name from a string.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

impl Deref for ActorName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<String> for ActorName {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for ActorName {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deref_gives_str() {
        // Given an ActorName.
        let name = ActorName::new("test-actor");

        // Then deref gives the inner string.
        assert_eq!(&*name, "test-actor");
    }

    #[test]
    fn from_string_and_str() {
        // Given string conversions.
        let from_string = ActorName::from(String::from("actor"));
        let from_str = ActorName::from("actor");

        // Then both produce the same name.
        assert_eq!(from_string, from_str);
    }
}
