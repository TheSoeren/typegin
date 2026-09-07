pub mod door_data;
pub mod interactions_data;
pub mod object_data;
pub mod room_data;

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::path::Path;

use serde::Deserialize;

use crate::data::interactions_data::{InteractionData, InteractionsFile};
use crate::data::object_data::{ObjectData, ObjectsFile};
use crate::data::room_data::{RoomData, RoomsFile};

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ExtraValue {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Array(Vec<ExtraValue>),
    Table(HashMap<String, ExtraValue>),
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorldData {
    pub objects: Vec<ObjectData>,
    pub rooms: Vec<RoomData>,
    pub interactions: Vec<InteractionData>,
}

impl WorldData {
    /// Look up an object definition by id, if present.
    #[must_use]
    pub fn find_object(&self, id: i32) -> Option<&ObjectData> {
        self.objects.iter().find(|object| object.id == id)
    }

    /// Look up a room definition by id, if present.
    #[must_use]
    pub fn find_room(&self, id: i32) -> Option<&RoomData> {
        self.rooms.iter().find(|room| room.id == id)
    }
}

#[derive(Debug)]
pub enum WorldDataError {
    Io(std::io::Error),
    Yaml(serde_yaml_ng::Error),
}

impl fmt::Display for WorldDataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorldDataError::Io(err) => write!(f, "failed to read data file: {err}"),
            WorldDataError::Yaml(err) => write!(f, "failed to parse data file: {err}"),
        }
    }
}

impl Error for WorldDataError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            WorldDataError::Io(err) => Some(err),
            WorldDataError::Yaml(err) => Some(err),
        }
    }
}

impl From<std::io::Error> for WorldDataError {
    fn from(err: std::io::Error) -> Self {
        WorldDataError::Io(err)
    }
}

impl From<serde_yaml_ng::Error> for WorldDataError {
    fn from(err: serde_yaml_ng::Error) -> Self {
        WorldDataError::Yaml(err)
    }
}

impl WorldData {
    /// Build the world data from raw YAML strings: the items file (`objects`),
    /// the rooms file (`rooms`) and the interactions file (`interactions`).
    ///
    /// # Errors
    ///
    /// Returns a [`serde_yaml_ng::Error`] if any YAML string is malformed or
    /// does not match the expected item/room/interaction shape.
    pub fn from_yaml(
        items_yaml: &str,
        rooms_yaml: &str,
        interactions_yaml: &str,
    ) -> Result<Self, serde_yaml_ng::Error> {
        let objects: ObjectsFile = serde_yaml_ng::from_str(items_yaml)?;
        let rooms: RoomsFile = serde_yaml_ng::from_str(rooms_yaml)?;
        let interactions: InteractionsFile = serde_yaml_ng::from_str(interactions_yaml)?;

        Ok(WorldData {
            objects: objects.objects,
            rooms: rooms.rooms,
            interactions: interactions.interactions,
        })
    }

    /// Load world data from the given items, rooms and interactions YAML
    /// files on disk.
    ///
    /// # Errors
    ///
    /// Returns a [`WorldDataError`] if any file cannot be read, or if any
    /// file's contents fail to parse as world data.
    pub fn load(
        items_path: impl AsRef<Path>,
        rooms_path: impl AsRef<Path>,
        interactions_path: impl AsRef<Path>,
    ) -> Result<Self, WorldDataError> {
        let items_yaml = std::fs::read_to_string(items_path)?;
        let rooms_yaml = std::fs::read_to_string(rooms_path)?;
        let interactions_yaml = std::fs::read_to_string(interactions_path)?;

        Ok(Self::from_yaml(
            &items_yaml,
            &rooms_yaml,
            &interactions_yaml,
        )?)
    }
}
