pub mod item;
pub mod player;
pub mod room;

use std::collections::HashMap;

use getset::{CopyGetters, Getters};
use log::warn;

use crate::data;
use crate::input::action;
use crate::input::direction;

#[derive(Debug, Getters, CopyGetters)]
pub struct WorldState {
    #[getset(get = "pub")]
    player: player::Player,
    #[getset(get = "pub")]
    rooms: HashMap<room::RoomId, room::Room>,
    #[getset(get_copy = "pub")]
    current_room_id: room::RoomId,
}

/// Navigation: location and movement within the world.
impl WorldState {
    /// The room the player is currently in.
    fn current_room(&self) -> &room::Room {
        self.rooms
            .get(&self.current_room_id)
            .expect("current room must be present in the world")
    }

    /// The mutable room the player is currently in.
    fn current_room_mut(&mut self) -> &mut room::Room {
        self.rooms
            .get_mut(&self.current_room_id)
            .expect("current room must be present in the world")
    }

    /// Resolve which room an open exit direction leads to from the current room.
    ///
    /// Hidden and locked exits are not traversable, so they resolve to `None`
    /// just like a direction with no exit at all.
    pub fn get_room_id_by_exit_direction(
        &self,
        direction: direction::Direction,
    ) -> Option<room::RoomId> {
        self.current_room().get_room_id_by_exit_direction(direction)
    }

    /// Change the current room to `room_id`, if it is known to the world.
    pub fn move_to_room(&mut self, room_id: room::RoomId) -> action::MoveResult {
        if self.rooms.contains_key(&room_id) {
            self.current_room_id = room_id;
            action::MoveResult::Success
        } else {
            warn!("Tried to move to unknown room (id: {})!", room_id);
            action::MoveResult::Fail
        }
    }

    /// Whether the exit in `direction` is locked (blocks traversal).
    pub fn is_exit_locked(&self, direction: direction::Direction) -> bool {
        self.current_room().is_exit_locked(direction)
    }

    /// Whether the exit in `direction` is hidden (the player is unaware of it).
    pub fn is_exit_hidden(&self, direction: direction::Direction) -> bool {
        self.current_room().is_exit_hidden(direction)
    }

    /// The opaque `extra` data attached to the exit in `direction`, if any.
    pub fn exit_extra(
        &self,
        direction: direction::Direction,
    ) -> Option<HashMap<String, data::ExtraValue>> {
        self.current_room().exit_extra(direction)
    }

    pub fn unlock_exit(
        &mut self,
        direction: direction::Direction,
    ) -> direction::DirectionResolution {
        self.current_room_mut().unlock_exit(direction)
    }

    pub fn lock_exit(&mut self, direction: direction::Direction) -> direction::DirectionResolution {
        self.current_room_mut().lock_exit(direction)
    }

    pub fn reveal_exit(
        &mut self,
        direction: direction::Direction,
    ) -> direction::DirectionResolution {
        self.current_room_mut().reveal_exit(direction)
    }

    pub fn hide_exit(&mut self, direction: direction::Direction) -> direction::DirectionResolution {
        self.current_room_mut().hide_exit(direction)
    }
}

/// Room helpers
impl WorldState {
    pub fn get_item_from_room(&self, id: item::ItemId) -> item::ItemResolution {
        self.current_room().get_item(id)
    }

    pub fn remove_item_from_room(&mut self, id: item::ItemId) -> Option<item::Item> {
        self.current_room_mut().remove_item(id)
    }

    /// Names of the items currently visible in the current room.
    pub fn room_item_names(&self) -> Vec<String> {
        self.current_room()
            .items()
            .iter()
            .map(|item| item.primary_name.clone())
            .collect()
    }

    pub fn reveal_item(&mut self, id: item::ItemId) -> item::ItemResolution {
        self.current_room_mut().reveal_item(id)
    }

    pub fn hide_item(&mut self, id: item::ItemId) -> item::ItemResolution {
        self.current_room_mut().hide_item(id)
    }

    pub fn current_room_extra(&self) -> HashMap<String, data::ExtraValue> {
        self.current_room().extra().clone()
    }
}

/// Player helpers
impl WorldState {
    pub fn get_item_from_player(&self, id: item::ItemId) -> item::ItemResolution {
        self.player.get_item(id)
    }

    pub fn remove_item_from_player(&mut self, id: item::ItemId) -> Option<item::Item> {
        self.player.remove_item(id)
    }

    /// Names of the items currently held by the player.
    pub fn player_item_names(&self) -> Vec<String> {
        self.player
            .items()
            .iter()
            .map(|item| item.primary_name.clone())
            .collect()
    }
}

/// Item transfer management
impl WorldState {
    pub fn player_take_item(&mut self, id: item::ItemId) -> action::TakeResult {
        let removed_item = self.remove_item_from_room(id);
        match removed_item {
            Some(item) => {
                self.player.add_item(item);
                action::TakeResult::Success
            }
            None => action::TakeResult::Fail,
        }
    }

    pub fn player_drop_item(&mut self, id: item::ItemId) -> action::DropResult {
        let removed_item = self.remove_item_from_player(id);
        match removed_item {
            Some(item) => {
                self.current_room_mut().add_item(item);
                action::DropResult::Success
            }
            None => action::DropResult::Fail,
        }
    }
}

/// All available items' helper methods
impl WorldState {
    /// Details about an item visible to the player (in the room or inventory).
    pub fn item_info(&self, id: item::ItemId) -> Option<item::ItemInfo> {
        self.get_available_items()
            .iter()
            .find(|item| item.id == id)
            .map(item::ItemInfo::from_item)
    }

    pub fn resolve_any_item(&self, name: &str) -> item::ItemResolution {
        let items = self.get_available_items();
        item::Item::resolve_item_by_name(&items, name)
    }

    pub fn resolve_player_item(&self, name: &str) -> item::ItemResolution {
        self.player.find_item(name)
    }

    pub fn resolve_room_item(&self, name: &str) -> item::ItemResolution {
        self.current_room().find_item(name)
    }

    fn get_available_items(&self) -> Vec<item::Item> {
        [
            self.current_room().items().as_slice(),
            self.player().items().as_slice(),
        ]
        .concat()
    }
}

impl WorldState {
    /// Build a `WorldState` directly from world data (TOML), with no database.
    pub(crate) fn from_data(data: &data::WorldData) -> Self {
        let first_room_id: room::RoomId = data
            .rooms
            .first()
            .expect("world data must contain at least one room")
            .id
            .into();

        let mut rooms = HashMap::new();
        for room_data in &data.rooms {
            let items: Vec<item::Item> = room_data
                .visible_items
                .iter()
                .filter_map(|id| data.find_item(*id))
                .map(item::Item::from_data)
                .collect();

            let hidden_items: Vec<item::Item> = room_data
                .hidden_items
                .iter()
                .filter_map(|id| data.find_item(*id))
                .map(item::Item::from_data)
                .collect();

            let exits = exits_from_data(room_data);

            rooms.insert(
                room_data.id.into(),
                room::Room::new(items, hidden_items, exits, room_data.extra.clone()),
            );
        }

        if !rooms.contains_key(&first_room_id) {
            panic!("first room id {first_room_id} not found in rooms");
        }

        WorldState {
            player: player::Player::new(),
            rooms,
            current_room_id: first_room_id,
        }
    }
}

/// Converts a `RoomData.exits` string map into a `Direction`-keyed map of
/// `Exit` values, carrying each exit's destination and state flags.
fn exits_from_data(room_data: &crate::data::RoomData) -> HashMap<direction::Direction, room::Exit> {
    room_data
        .exits
        .iter()
        .filter_map(|(raw, exit_data)| {
            direction::Direction::parse(raw).map(|direction| {
                (
                    direction,
                    room::Exit {
                        to: exit_data.to.into(),
                        locked: exit_data.locked,
                        hidden: exit_data.hidden,
                        extra: exit_data.extra.clone(),
                    },
                )
            })
        })
        .collect()
}
