use serde::Deserialize;

/// Optional door data on a [`Scene`](crate::object_data::ObjectKind::Scene) object.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct DoorData {
    #[serde(default)]
    pub direction: Option<String>,
    pub to: String,
    #[serde(default)]
    pub locked: bool,
}
