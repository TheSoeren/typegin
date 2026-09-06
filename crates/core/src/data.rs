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

/// Optional door data on a [`Scene`](ObjectKind::Scene) object.
///
/// A door is an ordinary object (id, names, `extra`) that additionally leads
/// somewhere: `direction` is the compass direction the door occupies in its
/// room, `to` the room it leads into. `locked` blocks traversal, `hidden` is
/// *not* stored here — hidden-ness is list membership (the door object lives in
/// the room's `hidden_objects` until revealed). `gated_by` optionally links the
/// door to the object that unlocks it; the unlock behaviour itself is a rule.
#[derive(Debug, Clone, Deserialize)]
pub struct DoorData {
    pub direction: String,
    pub to: i32,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub gated_by: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ObjectData {
    pub id: i32,
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

#[derive(Debug, Clone, Deserialize)]
pub struct RoomData {
    pub id: i32,
    #[serde(default)]
    pub visible_objects: Vec<i32>,
    #[serde(default)]
    pub hidden_objects: Vec<i32>,
    #[serde(default)]
    pub extra: HashMap<String, ExtraValue>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorldData {
    pub objects: Vec<ObjectData>,
    pub rooms: Vec<RoomData>,
}

impl WorldData {
    /// Look up an object definition by id, if present.
    pub fn find_object(&self, id: i32) -> Option<&ObjectData> {
        self.objects.iter().find(|object| object.id == id)
    }

    /// Look up a room definition by id, if present.
    pub fn find_room(&self, id: i32) -> Option<&RoomData> {
        self.rooms.iter().find(|room| room.id == id)
    }
}

#[derive(Debug, Deserialize)]
struct ItemsFile {
    objects: Vec<ObjectData>,
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
            objects: items.objects,
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
