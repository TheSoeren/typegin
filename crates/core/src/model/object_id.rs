use serde::Deserialize;

/// Identifier for a world object. All interactables share one id space.
///
/// The id is the object's stable symbolic `key` from the authored world data
/// (e.g. `iron-key`); numeric ids never appear in authored YAML.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct ObjectId(String);

impl ObjectId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        ObjectId(value.into())
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

impl From<&str> for ObjectId {
    fn from(value: &str) -> Self {
        ObjectId::new(value)
    }
}

impl From<String> for ObjectId {
    fn from(value: String) -> Self {
        ObjectId(value)
    }
}

impl From<ObjectId> for String {
    fn from(id: ObjectId) -> Self {
        id.0
    }
}

impl std::fmt::Display for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
