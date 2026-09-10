use std::collections::HashMap;

use serde::Deserialize;

use crate::data::ExtraValue;
use crate::model::object_id::ObjectId;
use crate::model::room_id::RoomId;

#[derive(Debug, Deserialize)]
pub(crate) struct RoomsFile {
    pub(crate) rooms: Vec<RoomData>,
}

#[derive(Debug, Clone, Deserialize)]
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
