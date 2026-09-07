use std::collections::HashMap;

use serde::Deserialize;

use crate::data::{ExtraValue, door_data::DoorData};

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

#[derive(Debug, Deserialize)]
pub(crate) struct ObjectsFile {
    pub(crate) objects: Vec<ObjectData>,
}
