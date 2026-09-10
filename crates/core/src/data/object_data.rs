use std::collections::HashMap;

use serde::Deserialize;

use crate::data::ExtraValue;
use crate::data::door_data::DoorData;
use crate::data::{WorldData, WorldDataError};
use crate::keys::object_id::ObjectId;
use crate::keys::room_id::RoomId;

/// The two object kinds of the Visionaire model:
///
/// * [`Item`](Self::Item) — an inventory object: portable, carried in
///   inventory, taken from a room (a key, a sword).
/// * [`Scene`](Self::Scene) — a scene object: stays in the world, is
///   clickable/examinable but not portable (furniture, fixtures — and every
///   door). A door is just a Scene object with optional `door` data.
///
/// The kind defaults to `Item` when omitted from world data.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Default)]
pub enum ObjectKind {
    #[default]
    Item,
    Scene,
}

/// A single object definition from world data (YAML).
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ObjectData {
    #[serde(rename = "key")]
    pub id: ObjectId,
    pub primary_name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub kind: ObjectKind,
    #[serde(default)]
    pub door: Option<DoorData>,
    #[serde(default)]
    pub extra: HashMap<String, ExtraValue>,
}

impl ObjectData {
    /// Verify that this object's door (if any) leads to a declared room key.
    ///
    /// # Errors
    ///
    /// Returns a [`WorldDataError::Validation`] naming the first unknown key.
    pub(crate) fn validate_references(&self, data: &WorldData) -> Result<(), WorldDataError> {
        if let Some(door) = &self.door {
            data.find_room(&RoomId::new(&door.to)).ok_or_else(|| {
                WorldDataError::Validation(format!(
                    "door `{}` references unknown room key `{}`",
                    self.id, door.to
                ))
            })?;
        }
        Ok(())
    }
}
