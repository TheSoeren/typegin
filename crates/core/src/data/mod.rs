pub mod door_data;
pub mod global_data;
pub mod interactions_data;
pub mod npc_data;
pub mod object_data;
pub mod room_data;

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::path::Path;

use serde::Deserialize;

use crate::data::global_data::GlobalFile;
use crate::data::interactions_data::{InteractionData, InteractionsFile};
use crate::data::npc_data::{NpcData, NpcsFile};
use crate::data::object_data::{ObjectData, ObjectsFile};
use crate::data::room_data::{RoomData, RoomsFile};
use crate::model::object_id::ObjectId;
use crate::model::room_id::RoomId;

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
    #[serde(default)]
    pub flags: Vec<String>,
    pub objects: Vec<ObjectData>,
    pub rooms: Vec<RoomData>,
    pub interactions: Vec<InteractionData>,
    pub npcs: Vec<NpcData>,
}

impl WorldData {
    /// Look up an object definition by id, if present.
    #[must_use]
    pub fn find_object(&self, id: &ObjectId) -> Option<&ObjectData> {
        self.objects.iter().find(|object| object.id == *id)
    }

    /// Look up a room definition by id, if present.
    #[must_use]
    pub fn find_room(&self, id: &RoomId) -> Option<&RoomData> {
        self.rooms.iter().find(|room| room.id == *id)
    }

    /// Check the world data for structural integrity: unique object and room
    /// keys, and every key reference (room memberships, door `to`/`gated_by`,
    /// interaction fields, and NPC room/dialogue references) resolving to a
    /// declared key.
    ///
    /// # Errors
    ///
    /// Returns a [`WorldDataError::Validation`] naming the first problem.
    fn validate(&self) -> Result<(), WorldDataError> {
        ensure_unique_keys(
            self.objects.iter().map(|object| &object.id),
            "object key `{key}` declared more than once",
        )?;
        ensure_unique_keys(
            self.rooms.iter().map(|room| &room.id),
            "room key `{key}` declared more than once",
        )?;

        for room in &self.rooms {
            for id in room.visible_objects.iter().chain(&room.hidden_objects) {
                self.find_object(id).ok_or_else(|| {
                    WorldDataError::Validation(format!(
                        "room `{}` references unknown object key `{}`",
                        room.id, id
                    ))
                })?;
            }
        }

        for object in &self.objects {
            if let Some(door) = &object.door {
                self.find_room(&RoomId::new(&door.to)).ok_or_else(|| {
                    WorldDataError::Validation(format!(
                        "door `{}` references unknown room key `{}`",
                        object.id, door.to
                    ))
                })?;
                if let Some(gated_by) = &door.gated_by {
                    self.find_object(&ObjectId::new(gated_by)).ok_or_else(|| {
                        WorldDataError::Validation(format!(
                            "door `{}` references unknown object key `{}`",
                            object.id, gated_by
                        ))
                    })?;
                }
            }
        }

        for interaction in &self.interactions {
            interaction.validate_references(self)?;
        }

        for npc in &self.npcs {
            npc.validate_references(self)?;
        }

        Ok(())
    }
}

/// Check a key iterator for duplicates, reporting the first clash.
fn ensure_unique_keys<'a, K, I>(keys: I, message: &'static str) -> Result<(), WorldDataError>
where
    K: Eq + std::hash::Hash + std::fmt::Display + 'a,
    I: Iterator<Item = &'a K>,
{
    let mut seen = std::collections::HashSet::new();
    for key in keys {
        if !seen.insert(key) {
            return Err(WorldDataError::Validation(
                message.replace("{key}", &key.to_string()),
            ));
        }
    }
    Ok(())
}

#[derive(Debug)]
pub enum WorldDataError {
    Io(std::io::Error),
    Yaml(serde_yaml_ng::Error),
    /// A key referenced by the world data does not exist, or a key is
    /// declared more than once.
    Validation(String),
}

impl fmt::Display for WorldDataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorldDataError::Io(err) => write!(f, "failed to read data file: {err}"),
            WorldDataError::Yaml(err) => write!(f, "failed to parse data file: {err}"),
            WorldDataError::Validation(message) => write!(f, "invalid world data: {message}"),
        }
    }
}

impl Error for WorldDataError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            WorldDataError::Io(err) => Some(err),
            WorldDataError::Yaml(err) => Some(err),
            WorldDataError::Validation(_) => None,
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
    /// Returns a [`WorldDataError`] if any YAML string is malformed or does
    /// not match the expected item/room/interaction shape, or if the world
    /// data is structurally invalid (duplicate keys, unknown key references).
    pub fn from_yaml(
        globals_yaml: &str,
        items_yaml: &str,
        rooms_yaml: &str,
        interactions_yaml: &str,
        npcs_yaml: &str,
    ) -> Result<Self, WorldDataError> {
        let globals: GlobalFile = serde_yaml_ng::from_str(globals_yaml)?;
        let objects: ObjectsFile = serde_yaml_ng::from_str(items_yaml)?;
        let rooms: RoomsFile = serde_yaml_ng::from_str(rooms_yaml)?;
        let interactions: InteractionsFile = serde_yaml_ng::from_str(interactions_yaml)?;
        let npcs: NpcsFile = serde_yaml_ng::from_str(npcs_yaml)?;

        let data = WorldData {
            flags: globals.flags,
            objects: objects.objects,
            rooms: rooms.rooms,
            interactions: interactions.interactions,
            npcs: npcs.npcs,
        };
        data.validate()?;

        Ok(data)
    }

    /// Load world data from the given items, rooms and interactions YAML
    /// files on disk.
    ///
    /// # Errors
    ///
    /// Returns a [`WorldDataError`] if any file cannot be read, or if any
    /// file's contents fail to parse as world data.
    pub fn load(
        globals_path: impl AsRef<Path>,
        items_path: impl AsRef<Path>,
        rooms_path: impl AsRef<Path>,
        interactions_path: impl AsRef<Path>,
        npcs_path: impl AsRef<Path>,
    ) -> Result<Self, WorldDataError> {
        let globals_yaml = std::fs::read_to_string(globals_path)?;
        let items_yaml = std::fs::read_to_string(items_path)?;
        let rooms_yaml = std::fs::read_to_string(rooms_path)?;
        let interactions_yaml = std::fs::read_to_string(interactions_path)?;
        let npcs_yaml = std::fs::read_to_string(npcs_path)?;

        Self::from_yaml(
            &globals_yaml,
            &items_yaml,
            &rooms_yaml,
            &interactions_yaml,
            &npcs_yaml,
        )
    }
}
