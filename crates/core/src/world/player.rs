use diesel::prelude::*;
use diesel::result::Error as DieselError;
use diesel::sqlite::SqliteConnection;
use getset::Getters;

use crate::EntityId;
use crate::schema::inventory_items;
use crate::schema::items;
use crate::schema::players;
use crate::world::item;

#[derive(Debug, Getters)]
#[getset(get = "pub(crate)")]
pub(crate) struct Player {
    items: Vec<item::Item>,
}

impl Player {
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
        self.items.push(item);
    }

    pub(crate) fn remove_item(&mut self, id: item::ItemId) -> Option<item::Item> {
        let item_position = self.items.iter().position(|item| item.id == id);
        match item_position {
            Some(pos) => Some(self.items.remove(pos)),
            None => None,
        }
    }
}

impl Player {
    pub(crate) fn load(conn: &mut SqliteConnection, id: EntityId) -> Result<Self, DieselError> {
        let (inventory_id,): (EntityId,) = players::table
            .find(id)
            .select((players::inventory_id,))
            .first(conn)?;

        let items: Vec<item::Item> = items::table
            .inner_join(inventory_items::table.on(inventory_items::item_id.eq(items::id)))
            .filter(inventory_items::inventory_id.eq(inventory_id))
            .select(item::Item::as_select())
            .load(conn)?;

        Ok(Player { items })
    }
}
