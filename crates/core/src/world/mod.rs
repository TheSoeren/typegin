mod item;
mod player;
mod room;

use std::collections::HashMap;

use diesel::prelude::{ExpressionMethods, QueryDsl, RunQueryDsl};
use diesel::result::Error as DieselError;
use diesel::sqlite::SqliteConnection;
use getset::{CopyGetters, Getters};
use log::warn;
use room::Room;

use crate::data::WorldData;
use crate::schema::{inventories, inventory_items, items, players, rooms, world_states};
pub use crate::world::item::ItemInfo;
use crate::{Direction, EntityId};

use crate::world::item::Item;
use crate::world::player::Player;

#[derive(Debug, Getters, CopyGetters)]
pub struct WorldState {
    #[getset(get = "pub")]
    player: Player,
    #[getset(get = "pub")]
    rooms: HashMap<EntityId, Room>,
    #[getset(get_copy = "pub")]
    current_room_id: EntityId,
}

/// Navigation: location and movement within the world.
impl WorldState {
    /// The room the player is currently in.
    ///
    /// The current room is always present in the world cache, so this never
    /// fails in practice.
    fn current_room(&self) -> &Room {
        self.rooms
            .get(&self.current_room_id)
            .expect("current room must be present in the world")
    }

    /// The mutable room the player is currently in.
    ///
    /// The current room is always present in the world cache, so this never
    /// fails in practice.
    fn current_room_mut(&mut self) -> &mut Room {
        self.rooms
            .get_mut(&self.current_room_id)
            .expect("current room must be present in the world")
    }

    /// Resolve which room an exit direction leads to from the current room.
    ///
    /// Read-only query: returns the target room id, or `None` if there is no
    /// exit in that direction. The actual room change is done by [`Self::move_to_room`].
    pub fn get_room_id_by_exit_direction(&self, direction: Direction) -> Option<EntityId> {
        self.current_room().exits().get(&direction).copied()
    }

    /// Change the current room to `room_id`, if it is known to the world.
    ///
    /// No-ops (and returns `false`) when `room_id` is unknown, so a malformed
    /// exit never crashes the game.
    pub fn move_to_room(&mut self, room_id: EntityId) -> bool {
        if self.rooms.contains_key(&room_id) {
            self.current_room_id = room_id;
            true
        } else {
            warn!("Tried to move to unknown room (id: {})!", room_id);
            false
        }
    }
}

/// Inventory & room item state, and entity resolution.
impl WorldState {
    /// Whether the player is currently holding the given item.
    pub fn player_has_item(&self, id: EntityId) -> bool {
        self.player.has_item(id)
    }

    /// Whether the given item is currently in the room (not the inventory).
    pub fn room_has_item(&self, id: EntityId) -> bool {
        self.current_room().has_item(id)
    }

    /// Names of the items currently visible in the current room.
    pub fn room_item_names(&self) -> Vec<String> {
        self.current_room()
            .items()
            .iter()
            .map(|item| item.primary_name.clone())
            .collect()
    }

    /// Names of the items currently held by the player.
    pub fn inventory_item_names(&self) -> Vec<String> {
        self.player
            .items()
            .iter()
            .map(|item| item.primary_name.clone())
            .collect()
    }

    pub fn move_item_to_inventory(&mut self, id: EntityId) -> bool {
        if self.player.has_item(id) {
            return false;
        }

        let room = self.current_room_mut();
        match room.remove_item(id) {
            Some(item) => {
                self.player.add_item(item);
                true
            }
            None => false,
        }
    }

    /// Drop an item from the player's inventory into the current room.
    ///
    /// Returns `false` if the item is not held, or is already present in the
    /// current room.
    pub fn move_item_from_inventory(&mut self, id: EntityId) -> bool {
        if self.current_room().has_item(id) {
            return false;
        }

        match self.player.remove_item(id) {
            Some(item) => {
                self.current_room_mut().add_item(item);
                true
            }
            None => false,
        }
    }
}

/// All available items' helper methods
impl WorldState {
    /// Item lookups by id (across room and inventory).
    pub fn get_item_name(&self, id: EntityId) -> Option<String> {
        self.get_available_items()
            .iter()
            .find(|item| item.id == id)
            .map(|item| item.primary_name.clone())
    }

    /// Details about an item visible to the player (in the room or inventory).
    pub fn item_info(&self, id: EntityId) -> Option<ItemInfo> {
        self.get_available_items()
            .iter()
            .find(|item| item.id == id)
            .map(ItemInfo::from_item)
    }

    pub fn resolve_any_item(&self, name: &str) -> Resolution {
        let room_items = self.get_available_items();
        self.resolve_entity(&room_items, name)
    }

    pub fn resolve_player_item(&self, name: &str) -> Resolution {
        let room_items = self.player().items();
        self.resolve_entity(room_items, name)
    }

    pub fn resolve_room_item(&self, name: &str) -> Resolution {
        let room_items = self.current_room().items();
        self.resolve_entity(room_items, name)
    }

    fn resolve_entity(&self, items: &[Item], name: &str) -> Resolution {
        let matching_ids: Vec<EntityId> = items
            .iter()
            .filter(|item| item.has_name(name))
            .map(|item| item.id)
            .collect();

        match matching_ids.len() {
            0 => Resolution::NotFound,
            1 => Resolution::Found(matching_ids[0]),
            _ => Resolution::Ambiguous(matching_ids),
        }
    }

    fn get_available_items(&self) -> Vec<Item> {
        [
            self.current_room().items().as_slice(),
            self.player().items().as_slice(),
        ]
        .concat()
    }
}

impl WorldState {
    pub(crate) fn load_or_seed(
        conn: &mut SqliteConnection,
        data: &WorldData,
    ) -> Result<Self, DieselError> {
        let count: i64 = world_states::table.count().get_result(conn)?;
        let world_id = if count == 0 {
            Self::seed(conn, data)?
        } else {
            world_states::table.select(world_states::id).first(conn)?
        };

        Self::load(conn, world_id, data)
    }

    pub(crate) fn seed(
        conn: &mut SqliteConnection,
        data: &WorldData,
    ) -> Result<EntityId, DieselError> {
        let first_room_id = data
            .rooms
            .first()
            .expect("world data must contain at least one room")
            .id;

        for room in &data.rooms {
            let inventory_id: EntityId = diesel::insert_into(inventories::table)
                .default_values()
                .returning(inventories::id)
                .get_result(conn)?;

            diesel::insert_into(rooms::table)
                .values((rooms::id.eq(room.id), rooms::inventory_id.eq(inventory_id)))
                .execute(conn)?;

            for item_id in &room.visible_items {
                diesel::insert_into(inventory_items::table)
                    .values((
                        inventory_items::inventory_id.eq(inventory_id),
                        inventory_items::item_id.eq(item_id),
                        inventory_items::hidden.eq(false),
                    ))
                    .execute(conn)?;
            }

            for item_id in &room.hidden_items {
                diesel::insert_into(inventory_items::table)
                    .values((
                        inventory_items::inventory_id.eq(inventory_id),
                        inventory_items::item_id.eq(item_id),
                        inventory_items::hidden.eq(true),
                    ))
                    .execute(conn)?;
            }
        }

        for item in &data.items {
            diesel::insert_into(items::table)
                .values((
                    items::id.eq(item.id),
                    items::primary_name.eq(&item.primary_name),
                    items::aliases.eq(item.aliases.join(";")),
                ))
                .execute(conn)?;
        }

        let player_inventory_id: EntityId = diesel::insert_into(inventories::table)
            .default_values()
            .returning(inventories::id)
            .get_result(conn)?;

        let player_id: EntityId = diesel::insert_into(players::table)
            .values(players::inventory_id.eq(player_inventory_id))
            .returning(players::id)
            .get_result(conn)?;

        diesel::insert_into(world_states::table)
            .values((
                world_states::player_id.eq(player_id),
                world_states::current_room_id.eq(first_room_id),
            ))
            .returning(world_states::id)
            .get_result(conn)
    }

    pub(crate) fn load(
        conn: &mut SqliteConnection,
        id: EntityId,
        data: &WorldData,
    ) -> Result<Self, DieselError> {
        let (player_id, current_room_id): (EntityId, EntityId) = world_states::table
            .find(id)
            .select((world_states::player_id, world_states::current_room_id))
            .first(conn)?;

        let player = Player::load(conn, player_id)?;

        let mut rooms = HashMap::new();
        for room_data in &data.rooms {
            let mut room = Room::load(conn, room_data.id)?;
            room.set_exits(exits_from_data(room_data));
            rooms.insert(room_data.id, room);
        }

        if !rooms.contains_key(&current_room_id) {
            return Err(DieselError::NotFound);
        }

        Ok(WorldState {
            player,
            rooms,
            current_room_id,
        })
    }
}

/// Converts a `RoomData.exits` string map into a `Direction`-keyed map.
fn exits_from_data(room_data: &crate::data::RoomData) -> HashMap<Direction, EntityId> {
    room_data
        .exits
        .iter()
        .filter_map(|(raw, target)| Direction::parse(raw).map(|direction| (direction, *target)))
        .collect()
}

#[derive(Debug, PartialEq, Eq)]
pub enum Resolution {
    Found(EntityId),
    Ambiguous(Vec<EntityId>),
    NotFound,
}
