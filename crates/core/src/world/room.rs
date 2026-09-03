use getset::{Getters, MutGetters, Setters};
use std::collections::HashMap;

use crate::input;
use crate::world::item;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RoomId(i32);

impl RoomId {
    pub fn new(value: i32) -> Self {
        RoomId(value)
    }

    pub fn get(self) -> i32 {
        self.0
    }
}

impl std::fmt::Display for RoomId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i32> for RoomId {
    fn from(value: i32) -> Self {
        RoomId::new(value)
    }
}

impl From<RoomId> for i32 {
    fn from(id: RoomId) -> i32 {
        id.get()
    }
}

#[derive(Debug, Getters, MutGetters, Setters, Default, Clone)]
#[getset(get = "pub(crate)")]
pub struct Room {
    #[get_mut(get_mut = "pub(crate)")]
    items: Vec<item::Item>,
    #[get_mut(get_mut = "pub(crate)")]
    hidden_items: Vec<item::Item>,
    #[getset(set = "pub(crate)", get_mut)]
    exits: HashMap<input::Direction, RoomId>,
    #[getset(set = "pub(crate)", get_mut)]
    hidden_exits: HashMap<input::Direction, RoomId>,
}

impl Room {
    pub(crate) fn new(
        items: Vec<item::Item>,
        hidden_items: Vec<item::Item>,
        exits: HashMap<input::Direction, RoomId>,
        hidden_exits: HashMap<input::Direction, RoomId>,
    ) -> Self {
        Room {
            items,
            hidden_items,
            exits,
            hidden_exits,
        }
    }
}

// Item management
impl Room {
    pub(crate) fn get_item(&self, id: item::ItemId) -> item::ItemResolution {
        match self.items.iter().find(|item| item.id == id) {
            Some(item) => item::ItemResolution::Found(item.id),
            None => item::ItemResolution::NotFound,
        }
    }

    pub(crate) fn find_item(&self, name: &str) -> item::ItemResolution {
        item::Item::resolve_item_by_name(self.items(), name)
    }

    pub(crate) fn add_item(&mut self, item: item::Item) {
        self.items_mut().push(item);
    }

    pub(crate) fn add_hidden_item(&mut self, item: item::Item) {
        self.hidden_items_mut().push(item);
    }

    pub(crate) fn remove_item(&mut self, id: item::ItemId) -> Option<item::Item> {
        Room::remove_item_from_list(self.items_mut(), id)
    }

    pub(crate) fn remove_hidden_item(&mut self, id: item::ItemId) -> Option<item::Item> {
        Room::remove_item_from_list(self.hidden_items_mut(), id)
    }

    fn remove_item_from_list(items: &mut Vec<item::Item>, id: item::ItemId) -> Option<item::Item> {
        let item_position = items.iter().position(|item| item.id == id);
        item_position.map(|pos| items.remove(pos))
    }

    pub(crate) fn reveal_item(&mut self, id: item::ItemId) -> item::ItemResolution {
        let removed_item = self.remove_hidden_item(id);
        match removed_item {
            Some(item) => {
                self.add_item(item);
                item::ItemResolution::Found(id)
            }
            None => item::ItemResolution::NotFound,
        }
    }

    pub(crate) fn hide_item(&mut self, id: item::ItemId) -> item::ItemResolution {
        let removed_item = self.remove_item(id);
        match removed_item {
            Some(item) => {
                self.add_hidden_item(item);
                item::ItemResolution::Found(id)
            }
            None => item::ItemResolution::NotFound,
        }
    }
}

// Exit management
impl Room {
    fn add_exit(&mut self, direction: input::Direction, room_id: RoomId) {
        self.exits_mut().insert(direction, room_id);
    }

    fn remove_exit(&mut self, direction: input::Direction) -> Option<RoomId> {
        self.exits_mut().remove(&direction)
    }

    fn add_hidden_exit(&mut self, direction: input::Direction, room_id: RoomId) {
        self.hidden_exits_mut().insert(direction, room_id);
    }

    fn remove_hidden_exit(&mut self, direction: input::Direction) -> Option<RoomId> {
        self.hidden_exits_mut().remove(&direction)
    }

    pub(crate) fn reveal_exit(
        &mut self,
        direction: input::Direction,
    ) -> crate::input::direction::DirectionResolution {
        let removed_exit = self.remove_hidden_exit(direction);
        match removed_exit {
            Some(id) => {
                self.add_exit(direction, id);
                crate::input::direction::DirectionResolution::Found(direction)
            }
            None => crate::input::direction::DirectionResolution::NotFound,
        }
    }

    pub(crate) fn hide_exit(
        &mut self,
        direction: input::Direction,
    ) -> crate::input::direction::DirectionResolution {
        let removed_exit = self.remove_exit(direction);
        match removed_exit {
            Some(id) => {
                self.add_hidden_exit(direction, id);
                crate::input::direction::DirectionResolution::Found(direction)
            }
            None => crate::input::direction::DirectionResolution::NotFound,
        }
    }
}
