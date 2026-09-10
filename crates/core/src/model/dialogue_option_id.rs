use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct DialogueOptionId(String);

impl DialogueOptionId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        DialogueOptionId(value.into())
    }

    /// The key as a string slice.
    #[must_use]
    pub fn get(&self) -> &str {
        &self.0
    }

    /// Consume the id, returning the key.
    #[must_use]
    pub fn into_key(self) -> String {
        self.0
    }
}

impl std::fmt::Display for DialogueOptionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for DialogueOptionId {
    fn from(value: &str) -> Self {
        DialogueOptionId::new(value)
    }
}

impl From<String> for DialogueOptionId {
    fn from(value: String) -> Self {
        DialogueOptionId(value)
    }
}

impl From<DialogueOptionId> for String {
    fn from(id: DialogueOptionId) -> Self {
        id.0
    }
}
