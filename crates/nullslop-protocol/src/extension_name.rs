//! Extension name newtype for source filtering.

use std::ops::Deref;

use derive_more::Display;

/// The name of an extension, used for source filtering.
///
/// When an extension sends a command or event, its name is attached as the
/// source so the extension host can avoid routing it back to the originator.
///
/// Implements [`Deref<Target = str>`] so it can be used directly in string
/// comparisons and formatting.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display)]
pub struct ExtensionName(String);

impl ExtensionName {
    /// Creates a new extension name from a string.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

impl Deref for ExtensionName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<String> for ExtensionName {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for ExtensionName {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deref_gives_str() {
        // Given an ExtensionName.
        let name = ExtensionName::new("test-ext");

        // Then deref gives the inner string.
        assert_eq!(&*name, "test-ext");
    }

    #[test]
    fn from_string_and_str() {
        // Given string conversions.
        let from_string = ExtensionName::from(String::from("ext"));
        let from_str = ExtensionName::from("ext");

        // Then both produce the same name.
        assert_eq!(from_string, from_str);
    }
}
