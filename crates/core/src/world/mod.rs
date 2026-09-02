pub mod item;
pub mod player;
pub mod room;

use std::collections::HashMap;

use diesel::prelude::{ExpressionMethods, QueryDsl, RunQueryDsl};
use diesel::result::Error as DieselError;
use diesel::sqlite::SqliteConnection;
use getset::{CopyGetters, Getters};
use log::warn;

use crate::data;
use crate::input::action;
use crate::input::direction::DirectionResolution;
use crate::schema;
use crate::{Direction, EntityId, RoomId};

#[derive(Debug, Getters, CopyGetters)]
pub struct WorldState {
    #[getset(get = "pub")]
    player: player::Player,
    #[getset(get = "pub")]
    rooms: HashMap<RoomId, room::Room>,
    #[getset(get_copy = "pub")]
    current_room_id: RoomId,
}

/// Navigation: location and movement within the world.
impl WorldState {
    /// The room the player is currently in.
    ///
    /// The current room is always present in the world cache, so this never
    /// fails in practice.
    fn current_room(&self) -> &room::Room {
        self.rooms
            .get(&self.current_room_id)
            .expect("current room must be present in the world")
    }

    /// The mutable room the player is currently in.
    ///
    /// The current room is always present in the world cache, so this never
    /// fails in practice.
    fn current_room_mut(&mut self) -> &mut room::Room {
        self.rooms
            .get_mut(&self.current_room_id)
            .expect("current room must be present in the world")
    }

    /// Resolve which room an exit direction leads to from the current room.
    ///
    /// Read-only query: returns the target room id, or `None` if there is no
    /// exit in that direction. The actual room change is done by [`Self::move_to_room`].
    pub fn get_room_id_by_exit_direction(&self, direction: Direction) -> Option<RoomId> {
        self.current_room().exits().get(&direction).copied()
    }

    /// Change the current room to `room_id`, if it is known to the world.
    ///
    /// No-ops (and returns [`MoveResult::Fail`]) when `room_id` is unknown, so
    /// a malformed exit never crashes the game.
    pub fn move_to_room(&mut self, room_id: RoomId) -> action::MoveResult {
        if self.rooms.contains_key(&room_id) {
            self.current_room_id = room_id;
            action::MoveResult::Success
        } else {
            warn!("Tried to move to unknown room (id: {})!", room_id);
            action::MoveResult::Fail
        }
    }

    pub fn hidden_exit_directions(&self) -> Vec<Direction> {
        self.current_room().hidden_exits().keys().copied().collect()
    }

    pub fn reveal_exit(&mut self, direction: Direction) -> DirectionResolution {
        self.current_room_mut().reveal_exit(direction)
    }

    pub fn hide_exit(&mut self, direction: Direction) -> DirectionResolution {
        self.current_room_mut().hide_exit(direction)
    }
}

/// Room item state
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
}

/// player item state
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
    pub(crate) fn load_or_seed(
        conn: &mut SqliteConnection,
        data: &data::WorldData,
    ) -> Result<Self, DieselError> {
        let count: i64 = schema::world_states::table.count().get_result(conn)?;
        let world_id = if count == 0 {
            Self::seed(conn, data)?
        } else {
            schema::world_states::table
                .select(schema::world_states::id)
                .first(conn)?
        };

        Self::load(conn, world_id, data)
    }

    pub(crate) fn seed(
        conn: &mut SqliteConnection,
        data: &data::WorldData,
    ) -> Result<EntityId, DieselError> {
        let first_room_id: RoomId = data
            .rooms
            .first()
            .expect("world data must contain at least one room")
            .id
            .into();

        for room in &data.rooms {
            let inventory_id: EntityId = diesel::insert_into(schema::inventories::table)
                .default_values()
                .returning(schema::inventories::id)
                .get_result(conn)?;

            diesel::insert_into(schema::rooms::table)
                .values((
                    schema::rooms::id.eq(room.id),
                    schema::rooms::inventory_id.eq(inventory_id),
                ))
                .execute(conn)?;

            for item_id in &room.visible_items {
                diesel::insert_into(schema::inventory_items::table)
                    .values((
                        schema::inventory_items::inventory_id.eq(inventory_id),
                        schema::inventory_items::item_id.eq(item_id),
                        schema::inventory_items::hidden.eq(false),
                    ))
                    .execute(conn)?;
            }

            for item_id in &room.hidden_items {
                diesel::insert_into(schema::inventory_items::table)
                    .values((
                        schema::inventory_items::inventory_id.eq(inventory_id),
                        schema::inventory_items::item_id.eq(item_id),
                        schema::inventory_items::hidden.eq(true),
                    ))
                    .execute(conn)?;
            }
        }

        for item in &data.items {
            diesel::insert_into(schema::items::table)
                .values((
                    schema::items::id.eq(item.id),
                    schema::items::primary_name.eq(&item.primary_name),
                    schema::items::aliases.eq(item.aliases.join(";")),
                ))
                .execute(conn)?;
        }

        let player_inventory_id: EntityId = diesel::insert_into(schema::inventories::table)
            .default_values()
            .returning(schema::inventories::id)
            .get_result(conn)?;

        let player_id: EntityId = diesel::insert_into(schema::players::table)
            .values(schema::players::inventory_id.eq(player_inventory_id))
            .returning(schema::players::id)
            .get_result(conn)?;

        diesel::insert_into(schema::world_states::table)
            .values((
                schema::world_states::player_id.eq(player_id),
                schema::world_states::current_room_id.eq(first_room_id),
            ))
            .returning(schema::world_states::id)
            .get_result(conn)
    }

    pub(crate) fn load(
        conn: &mut SqliteConnection,
        id: EntityId,
        data: &data::WorldData,
    ) -> Result<Self, DieselError> {
        let (player_id, current_room_id): (EntityId, RoomId) = schema::world_states::table
            .find(id)
            .select((
                schema::world_states::player_id,
                schema::world_states::current_room_id,
            ))
            .first(conn)?;

        let player = player::Player::load(conn, player_id)?;

        let mut rooms = HashMap::new();
        for room_data in &data.rooms {
            let mut room = room::Room::load(conn, room_data.id.into())?;
            room.set_exits(exits_from_data(room_data));
            room.set_hidden_exits(hidden_exits_from_data(room_data));
            rooms.insert(room_data.id.into(), room);
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
fn exits_from_data(room_data: &crate::data::RoomData) -> HashMap<Direction, RoomId> {
    room_data
        .exits
        .iter()
        .filter_map(|(raw, target)| {
            Direction::parse(raw).map(|direction| (direction, (*target).into()))
        })
        .collect()
}

/// Converts a `RoomData.hidden_exits` string map into a `Direction`-keyed map.
fn hidden_exits_from_data(room_data: &crate::data::RoomData) -> HashMap<Direction, RoomId> {
    room_data
        .hidden_exits
        .iter()
        .filter_map(|(raw, target)| {
            Direction::parse(raw).map(|direction| (direction, (*target).into()))
        })
        .collect()
}
