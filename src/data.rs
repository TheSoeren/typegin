use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct ItemData {
    pub(crate) id: i32,
    pub(crate) primary_name: String,
    #[serde(default)]
    pub(crate) aliases: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RoomData {
    pub(crate) id: i32,
    #[serde(default)]
    pub(crate) visible_items: Vec<i32>,
    #[serde(default)]
    pub(crate) hidden_items: Vec<i32>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WorldData {
    pub(crate) items: Vec<ItemData>,
    pub(crate) rooms: Vec<RoomData>,
}

#[derive(Debug, Deserialize)]
struct ItemsFile {
    items: Vec<ItemData>,
}

#[derive(Debug, Deserialize)]
struct RoomsFile {
    rooms: Vec<RoomData>,
}

pub(crate) fn load_world_data() -> WorldData {
    let items_file: ItemsFile =
        toml::from_str(include_str!("../data/items.toml")).expect("parse data/items.toml");
    let rooms_file: RoomsFile =
        toml::from_str(include_str!("../data/rooms.toml")).expect("parse data/rooms.toml");

    WorldData {
        items: items_file.items,
        rooms: rooms_file.rooms,
    }
}

