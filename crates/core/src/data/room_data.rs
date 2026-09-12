use std::collections::{HashMap, HashSet};

use serde::Deserialize;

use crate::Direction;
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

    /// Verify that no two of this room's doors (visible or hidden) occupy the
    /// same compass direction.
    ///
    /// Unknown object keys are not reported here — `validate_references`
    /// (run first, in `WorldData::validate`) already reports those — so a
    /// reference that doesn't resolve is silently skipped. An unparsable or
    /// absent direction string doesn't occupy a compass slot at all (it's a
    /// direction-less, point-and-click-only exit, per
    /// [`Object::from_data`](crate::world::object::Object::from_data)), so any
    /// number of those can coexist in one room without conflict.
    ///
    /// # Errors
    ///
    /// Returns a [`WorldDataError::Validation`] naming the first direction
    /// claimed by more than one door.
    pub(crate) fn validate_unique_directions(
        &self,
        data: &WorldData,
    ) -> Result<(), WorldDataError> {
        let mut seen_directions = HashSet::new();
        for id in self.visible_objects.iter().chain(&self.hidden_objects) {
            let Some(object) = data.find_object(id) else {
                continue;
            };
            let Some(direction) = object
                .door
                .as_ref()
                .and_then(|door| door.direction.as_deref())
                .and_then(Direction::parse)
            else {
                continue;
            };

            if !seen_directions.insert(direction) {
                return Err(WorldDataError::Validation(format!(
                    "room `{}` declares more than one exit in direction `{direction}`",
                    self.id
                )));
            }
        }
        Ok(())
    }
}
