use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct NpcId(String);

impl NpcId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        NpcId(value.into())
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

impl std::fmt::Display for NpcId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for NpcId {
    fn from(value: &str) -> Self {
        NpcId::new(value)
    }
}

impl From<String> for NpcId {
    fn from(value: String) -> Self {
        NpcId(value)
    }
}

impl From<NpcId> for String {
    fn from(id: NpcId) -> Self {
        id.0
    }
}
