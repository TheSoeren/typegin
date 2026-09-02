use std::collections::HashMap;

use diesel::AsExpression;
use diesel::backend::Backend;
use diesel::deserialize;
use diesel::prelude::*;
use diesel::result::Error as DieselError;
use diesel::serialize;
use diesel::sql_types::Integer;
use diesel::sqlite::Sqlite;
use diesel::sqlite::SqliteConnection;
use getset::Getters;
use getset::MutGetters;
use getset::Setters;

use crate::Direction;
use crate::EntityId;
use crate::ItemResolution;
use crate::input::direction;
use crate::schema::inventory_items;
use crate::schema::items;
use crate::schema::rooms;
use crate::world::item;

/// Numeric identifier for a room.
///
/// A thin wrapper over the database `rooms.id`, mirroring [`ItemId`](crate::ItemId)
/// so room references are type-distinct from other entity ids at the API edge.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, AsExpression, deserialize::FromSqlRow,
)]
#[diesel(sql_type = Integer)]
pub struct RoomId(i32);

impl RoomId {
    /// Build a `RoomId` from its raw database value.
    pub fn new(value: i32) -> Self {
        RoomId(value)
    }

    /// The raw database value backing this id.
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

impl deserialize::FromSql<Integer, Sqlite> for RoomId {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let val = i32::from_sql(bytes)?;
        Ok(RoomId(val))
    }
}

impl serialize::ToSql<Integer, Sqlite> for RoomId {
    fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, Sqlite>) -> serialize::Result {
        serialize::ToSql::<Integer, Sqlite>::to_sql(&self.0, out)
    }
}

#[derive(Debug, Getters, MutGetters, Setters, Default, Clone)]
#[getset(get = "pub(crate)")]
pub struct Room {
    id: RoomId,
    #[get_mut(get_mut = "pub(crate)")]
    items: Vec<item::Item>,
    #[get_mut(get_mut = "pub(crate)")]
    hidden_items: Vec<item::Item>,
    #[getset(set = "pub(crate)", get_mut)]
    exits: HashMap<Direction, RoomId>,
    #[getset(set = "pub(crate)", get_mut)]
    hidden_exits: HashMap<Direction, RoomId>,
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
                ItemResolution::Found(id)
            }
            None => ItemResolution::NotFound,
        }
    }

    pub(crate) fn hide_item(&mut self, id: item::ItemId) -> item::ItemResolution {
        let removed_item = self.remove_item(id);
        match removed_item {
            Some(item) => {
                self.add_hidden_item(item);
                ItemResolution::Found(id)
            }
            None => ItemResolution::NotFound,
        }
    }
}

// Exit management
impl Room {
    fn add_exit(&mut self, direction: Direction, room_id: RoomId) {
        self.exits_mut().insert(direction, room_id);
    }

    fn remove_exit(&mut self, direction: Direction) -> Option<RoomId> {
        self.exits_mut().remove(&direction)
    }

    fn add_hidden_exit(&mut self, direction: Direction, room_id: RoomId) {
        self.hidden_exits_mut().insert(direction, room_id);
    }

    fn remove_hidden_exit(&mut self, direction: Direction) -> Option<RoomId> {
        self.hidden_exits_mut().remove(&direction)
    }

    pub(crate) fn reveal_exit(&mut self, direction: Direction) -> direction::DirectionResolution {
        let removed_exit = self.remove_hidden_exit(direction);
        match removed_exit {
            Some(id) => {
                self.add_exit(direction, id);
                direction::DirectionResolution::Found(direction)
            }
            None => direction::DirectionResolution::NotFound,
        }
    }

    pub(crate) fn hide_exit(&mut self, direction: Direction) -> direction::DirectionResolution {
        let removed_exit = self.remove_exit(direction);
        match removed_exit {
            Some(id) => {
                self.add_hidden_exit(direction, id);
                direction::DirectionResolution::Found(direction)
            }
            None => direction::DirectionResolution::NotFound,
        }
    }
}

// Data management
impl Room {
    pub(crate) fn load(conn: &mut SqliteConnection, id: RoomId) -> Result<Self, DieselError> {
        let (inventory_id,): (EntityId,) = rooms::table
            .find(id)
            .select((rooms::inventory_id,))
            .first(conn)?;

        let items: Vec<item::Item> = items::table
            .inner_join(inventory_items::table.on(inventory_items::item_id.eq(items::id)))
            .filter(inventory_items::inventory_id.eq(inventory_id))
            .filter(inventory_items::hidden.eq(false))
            .select(item::Item::as_select())
            .load(conn)?;

        let hidden_items: Vec<item::Item> = items::table
            .inner_join(inventory_items::table.on(inventory_items::item_id.eq(items::id)))
            .filter(inventory_items::inventory_id.eq(inventory_id))
            .filter(inventory_items::hidden.eq(true))
            .select(item::Item::as_select())
            .load(conn)?;

        // Exits are not loaded from the DB yet (see WorldState notes); they
        // are populated from WorldData when room loading is wired up.
        let exits = HashMap::new();

        Ok(Room {
            id,
            items,
            hidden_items,
            exits,
            hidden_exits: HashMap::new(),
        })
    }
}
