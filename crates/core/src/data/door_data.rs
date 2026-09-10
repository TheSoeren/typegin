use serde::Deserialize;

/// Optional door data on a [`Scene`](ObjectKind::Scene) object.
#[derive(Debug, Clone, Deserialize)]
pub struct DoorData {
    pub direction: String,
    pub to: String,
    #[serde(default)]
    pub locked: bool,
}
