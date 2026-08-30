use diesel::prelude::*;
use diesel::result::Error as DieselError;
use diesel::sqlite::SqliteConnection;
use getset::Getters;

use crate::EntityId;
use crate::schema::inventory_items;
use crate::schema::items;
use crate::schema::rooms;

use super::item::Item;

#[derive(Debug, Getters)]
#[getset(get = "pub(crate)")]
pub struct Room {
    id: EntityId,
    items: Vec<Item>,
    hidden_items: Vec<Item>,
}

impl Room {
    pub(crate) fn load(conn: &mut SqliteConnection, id: EntityId) -> Result<Self, DieselError> {
        let (inventory_id,): (EntityId,) = rooms::table
            .find(id)
            .select((rooms::inventory_id,))
            .first(conn)?;

        let items: Vec<Item> = items::table
            .inner_join(inventory_items::table.on(inventory_items::item_id.eq(items::id)))
            .filter(inventory_items::inventory_id.eq(inventory_id))
            .filter(inventory_items::hidden.eq(false))
            .select(Item::as_select())
            .load(conn)?;

        let hidden_items: Vec<Item> = items::table
            .inner_join(inventory_items::table.on(inventory_items::item_id.eq(items::id)))
            .filter(inventory_items::inventory_id.eq(inventory_id))
            .filter(inventory_items::hidden.eq(true))
            .select(Item::as_select())
            .load(conn)?;

        Ok(Room {
            id,
            items,
            hidden_items,
        })
    }

    pub(crate) fn has_item(&self, item_id: EntityId) -> bool {
        self.items.iter().any(|item| item.id == item_id)
    }

    pub(crate) fn add_item(&mut self, item: Item) {
        self.items.push(item);
    }

    pub(crate) fn remove_item(&mut self, item_id: EntityId) -> Option<Item> {
        self.items
            .iter()
            .position(|item| item.id == item_id)
            .map(|index| self.items.remove(index))
    }
}
