use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::path::Path;

use serde::Deserialize;

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
pub struct ItemData {
    pub id: i32,
    pub primary_name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub extra: HashMap<String, ExtraValue>,
}

/// A single exit from a room as declared in world data.
#[derive(Debug, Clone, Deserialize)]
pub struct ExitData {
    pub to: i32,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub extra: HashMap<String, ExtraValue>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoomData {
    pub id: i32,
    #[serde(default)]
    pub visible_items: Vec<i32>,
    #[serde(default)]
    pub hidden_items: Vec<i32>,
    #[serde(default)]
    pub exits: HashMap<String, ExitData>,
    #[serde(default)]
    pub extra: HashMap<String, ExtraValue>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorldData {
    pub items: Vec<ItemData>,
    pub rooms: Vec<RoomData>,
}

impl WorldData {
    /// Look up an item definition by id, if present.
    pub fn find_item(&self, id: i32) -> Option<&ItemData> {
        self.items.iter().find(|item| item.id == id)
    }

    /// Look up a room definition by id, if present.
    pub fn find_room(&self, id: i32) -> Option<&RoomData> {
        self.rooms.iter().find(|room| room.id == id)
    }
}

#[derive(Debug, Deserialize)]
struct ItemsFile {
    items: Vec<ItemData>,
}

#[derive(Debug, Deserialize)]
struct RoomsFile {
    rooms: Vec<RoomData>,
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
    pub fn from_yaml(items_yaml: &str, rooms_yaml: &str) -> Result<Self, serde_yaml_ng::Error> {
        let items: ItemsFile = serde_yaml_ng::from_str(items_yaml)?;
        let rooms: RoomsFile = serde_yaml_ng::from_str(rooms_yaml)?;

        Ok(WorldData {
            items: items.items,
            rooms: rooms.rooms,
        })
    }

    pub fn load(
        items_path: impl AsRef<Path>,
        rooms_path: impl AsRef<Path>,
    ) -> Result<Self, WorldDataError> {
        let items_yaml = std::fs::read_to_string(items_path)?;
        let rooms_yaml = std::fs::read_to_string(rooms_path)?;

        Ok(Self::from_yaml(&items_yaml, &rooms_yaml)?)
    }
}
