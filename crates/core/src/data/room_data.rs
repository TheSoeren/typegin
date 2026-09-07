use std::collections::HashMap;

use serde::Deserialize;

use crate::data::ExtraValue;

#[derive(Debug, Deserialize)]
pub(crate) struct RoomsFile {
    pub(crate) rooms: Vec<RoomData>,
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
