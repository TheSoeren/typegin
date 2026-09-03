use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ItemData {
    pub id: i32,
    pub primary_name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoomData {
    pub id: i32,
    #[serde(default)]
    pub visible_items: Vec<i32>,
    #[serde(default)]
    pub hidden_items: Vec<i32>,
    #[serde(default)]
    pub exits: HashMap<String, i32>,
    #[serde(default)]
    pub hidden_exits: HashMap<String, i32>,
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
    Toml(toml::de::Error),
}

impl fmt::Display for WorldDataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorldDataError::Io(err) => write!(f, "failed to read data file: {err}"),
            WorldDataError::Toml(err) => write!(f, "failed to parse data file: {err}"),
        }
    }
}

impl Error for WorldDataError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            WorldDataError::Io(err) => Some(err),
            WorldDataError::Toml(err) => Some(err),
        }
    }
}

impl From<std::io::Error> for WorldDataError {
    fn from(err: std::io::Error) -> Self {
        WorldDataError::Io(err)
    }
}

impl From<toml::de::Error> for WorldDataError {
    fn from(err: toml::de::Error) -> Self {
        WorldDataError::Toml(err)
    }
}

impl WorldData {
    pub fn from_toml(items_toml: &str, rooms_toml: &str) -> Result<Self, toml::de::Error> {
        let items: ItemsFile = toml::from_str(items_toml)?;
        let rooms: RoomsFile = toml::from_str(rooms_toml)?;

        Ok(WorldData {
            items: items.items,
            rooms: rooms.rooms,
        })
    }

    pub fn load(
        items_path: impl AsRef<Path>,
        rooms_path: impl AsRef<Path>,
    ) -> Result<Self, WorldDataError> {
        let items_toml = std::fs::read_to_string(items_path)?;
        let rooms_toml = std::fs::read_to_string(rooms_path)?;

        Ok(Self::from_toml(&items_toml, &rooms_toml)?)
    }
}
