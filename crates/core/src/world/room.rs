use getset::{Getters, MutGetters};
use std::collections::HashMap;

use crate::data;
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

/// A single exit from a room: where it leads plus its independent state flags.
///
/// `locked` and `hidden` are independent — an exit may be both (a secret door
/// that also needs a key). Both block movement; the engine reports the reason
/// through distinct events (`WentExitHidden` vs `WentExitLocked`), so the
/// consumer decides whether a hidden exit is discoverable by trying, or
/// simply reads as a dead end.
#[derive(Debug, Clone, PartialEq)]
pub struct Exit {
    pub(crate) to: RoomId,
    pub(crate) locked: bool,
    pub(crate) hidden: bool,
    pub(crate) extra: HashMap<String, data::ExtraValue>,
}

#[derive(Debug, Getters, MutGetters, Default, Clone)]
#[getset(get = "pub(crate)")]
pub struct Room {
    #[get_mut(get_mut = "pub(crate)")]
    items: Vec<item::Item>,
    #[get_mut(get_mut = "pub(crate)")]
    hidden_items: Vec<item::Item>,
    exits: HashMap<input::Direction, Exit>,
    extra: HashMap<String, data::ExtraValue>,
}

impl Room {
    pub(crate) fn new(
        items: Vec<item::Item>,
        hidden_items: Vec<item::Item>,
        exits: HashMap<input::Direction, Exit>,
        extra: HashMap<String, data::ExtraValue>,
    ) -> Self {
        Room {
            items,
            hidden_items,
            exits,
            extra,
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
    /// The destination of an *open* exit in `direction`, if one exists.
    ///
    /// Hidden and locked exits are not usable for movement, so they resolve to
    /// `None` (exactly as if no exit were present).
    pub(crate) fn get_room_id_by_exit_direction(
        &self,
        direction: input::Direction,
    ) -> Option<RoomId> {
        self.exits
            .get(&direction)
            .filter(|exit| !exit.locked && !exit.hidden)
            .map(|exit| exit.to)
    }

    pub(crate) fn is_exit_locked(&self, direction: input::Direction) -> bool {
        self.exits.get(&direction).is_some_and(|exit| exit.locked)
    }

    pub(crate) fn is_exit_hidden(&self, direction: input::Direction) -> bool {
        self.exits.get(&direction).is_some_and(|exit| exit.hidden)
    }

    /// Directions leading to an *open* (passable) exit in this room.
    pub(crate) fn exit_directions(&self) -> Vec<input::Direction> {
        self.exits
            .iter()
            .filter(|(_, exit)| !exit.locked && !exit.hidden)
            .map(|(direction, _)| *direction)
            .collect()
    }

    pub(crate) fn exit_extra(
        &self,
        direction: input::Direction,
    ) -> Option<HashMap<String, data::ExtraValue>> {
        self.exits.get(&direction).map(|exit| exit.extra.clone())
    }

    pub(crate) fn lock_exit(&mut self, direction: input::Direction) -> input::DirectionResolution {
        match self.exits.get_mut(&direction) {
            Some(exit) if !exit.locked => {
                exit.locked = true;
                input::DirectionResolution::Found(direction)
            }
            _ => input::DirectionResolution::NotFound,
        }
    }

    pub(crate) fn unlock_exit(
        &mut self,
        direction: input::Direction,
    ) -> input::DirectionResolution {
        match self.exits.get_mut(&direction) {
            Some(exit) if exit.locked => {
                exit.locked = false;
                input::DirectionResolution::Found(direction)
            }
            _ => input::DirectionResolution::NotFound,
        }
    }

    pub(crate) fn hide_exit(&mut self, direction: input::Direction) -> input::DirectionResolution {
        match self.exits.get_mut(&direction) {
            Some(exit) if !exit.hidden => {
                exit.hidden = true;
                input::DirectionResolution::Found(direction)
            }
            _ => input::DirectionResolution::NotFound,
        }
    }

    pub(crate) fn reveal_exit(
        &mut self,
        direction: input::Direction,
    ) -> input::DirectionResolution {
        match self.exits.get_mut(&direction) {
            Some(exit) if exit.hidden => {
                exit.hidden = false;
                input::DirectionResolution::Found(direction)
            }
            _ => input::DirectionResolution::NotFound,
        }
    }
}
