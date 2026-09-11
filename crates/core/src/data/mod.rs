pub mod door_data;
pub mod interactions_data;
pub mod npc_data;
pub mod object_data;
pub mod room_data;
pub mod trigger_data;

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::path::Path;

use serde::Deserialize;

use crate::TriggerData;
use crate::data::interactions_data::InteractionData;
use crate::data::npc_data::NpcData;
use crate::data::object_data::ObjectData;
use crate::data::room_data::RoomData;
use crate::keys::object_id::ObjectId;
use crate::keys::room_id::RoomId;

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
    pub triggers: Vec<TriggerData>,
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
    /// keys, and every key reference (room memberships, door `to`, interaction
    /// fields, and NPC room/dialogue references) resolving to a declared key.
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
        ensure_unique_keys(
            self.triggers.iter().map(|trigger| &trigger.id),
            "trigger key `{key}` declared more than once",
        )?;

        if self.rooms.is_empty() {
            return Err(WorldDataError::Validation(
                "world data must declare at least one room".to_string(),
            ));
        }

        for room in &self.rooms {
            room.validate_references(self)?;
        }

        for object in &self.objects {
            object.validate_references(self)?;
        }

        for interaction in &self.interactions {
            interaction.validate_references(self)?;
        }

        for npc in &self.npcs {
            npc.validate_references(self)?;
        }

        for trigger in &self.triggers {
            trigger.validate_references(self)?;
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

/// The top-level shape of a world-data YAML document: every section as a
/// top-level key in one document. `objects` and `rooms` are required; the
/// rest default to empty when their key is omitted.
///
/// A consumer authoring content across multiple files (as the shipped
/// `data/*.yaml` files do) concatenates them into one string before calling
/// [`WorldData::from_yaml`] — each file already contributes disjoint
/// top-level keys, so concatenation is a lossless merge with no deep-merge
/// logic required. The core crate only ever sees one document; it has no
/// opinion on how many files a consumer's content is split across.
#[derive(Debug, Deserialize)]
struct WorldDataFile {
    #[serde(default)]
    flags: Vec<String>,
    objects: Vec<ObjectData>,
    rooms: Vec<RoomData>,
    #[serde(default)]
    interactions: Vec<InteractionData>,
    #[serde(default)]
    npcs: Vec<NpcData>,
    #[serde(default)]
    triggers: Vec<TriggerData>,
}

impl WorldData {
    /// Build the world data from a single YAML document (see
    /// [`WorldDataFile`] for its shape).
    ///
    /// # Errors
    ///
    /// Returns a [`WorldDataError`] if the YAML is malformed or does not
    /// match the expected shape, or if the world data is structurally
    /// invalid (duplicate keys, unknown key references).
    pub fn from_yaml(yaml: &str) -> Result<Self, WorldDataError> {
        let file: WorldDataFile = serde_yaml_ng::from_str(yaml)?;

        let data = WorldData {
            flags: file.flags,
            objects: file.objects,
            rooms: file.rooms,
            interactions: file.interactions,
            npcs: file.npcs,
            triggers: file.triggers,
        };
        data.validate()?;

        Ok(data)
    }

    /// Load world data from a single YAML file on disk.
    ///
    /// # Errors
    ///
    /// Returns a [`WorldDataError`] if the file cannot be read, or if its
    /// contents fail to parse as world data.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, WorldDataError> {
        let yaml = std::fs::read_to_string(path)?;
        Self::from_yaml(&yaml)
    }
}
