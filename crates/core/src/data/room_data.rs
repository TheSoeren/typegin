use std::collections::HashMap;

use serde::Deserialize;

use crate::data::ExtraValue;
use crate::data::{WorldData, WorldDataError};
use crate::keys::object_id::ObjectId;
use crate::keys::room_id::RoomId;

/// A single room definition from world data (YAML).
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct RoomData {
    #[serde(rename = "key")]
    pub id: RoomId,
    #[serde(default)]
    pub visible_objects: Vec<ObjectId>,
    #[serde(default)]
    pub hidden_objects: Vec<ObjectId>,
    #[serde(default)]
    pub extra: HashMap<String, ExtraValue>,
}

impl RoomData {
    /// Verify that every object this room contains (visible or hidden)
    /// references a declared object key.
    ///
    /// # Errors
    ///
    /// Returns a [`WorldDataError::Validation`] naming the first unknown key.
    pub(crate) fn validate_references(&self, data: &WorldData) -> Result<(), WorldDataError> {
        for id in self.visible_objects.iter().chain(&self.hidden_objects) {
            data.find_object(id).ok_or_else(|| {
                WorldDataError::Validation(format!(
                    "room `{}` references unknown object key `{}`",
                    self.id, id
                ))
            })?;
        }
        Ok(())
    }
}
