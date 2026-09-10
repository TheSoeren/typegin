use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct RoomId(String);

impl RoomId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        RoomId(value.into())
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

impl std::fmt::Display for RoomId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for RoomId {
    fn from(value: &str) -> Self {
        RoomId::new(value)
    }
}

impl From<String> for RoomId {
    fn from(value: String) -> Self {
        RoomId(value)
    }
}

impl From<RoomId> for String {
    fn from(id: RoomId) -> Self {
        id.0
    }
}
