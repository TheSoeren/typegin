use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct DialogueNodeId(String);

impl DialogueNodeId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        DialogueNodeId(value.into())
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

impl std::fmt::Display for DialogueNodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for DialogueNodeId {
    fn from(value: &str) -> Self {
        DialogueNodeId::new(value.to_string())
    }
}

impl From<String> for DialogueNodeId {
    fn from(value: String) -> Self {
        DialogueNodeId(value)
    }
}

impl From<DialogueNodeId> for String {
    fn from(id: DialogueNodeId) -> Self {
        id.0
    }
}
