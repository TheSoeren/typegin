use diesel::prelude::*;

use crate::EntityId;
use crate::schema::items;

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = items)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct Item {
    pub(crate) id: EntityId,
    pub(crate) primary_name: String,
    pub(crate) aliases: String,
}

impl Item {
    pub fn has_name(&self, name: &str) -> bool {
        self.primary_name == name || self.aliases.split(';').any(|alias| alias == name)
    }
}

/// Public, plain-data view of an item in the world, handed to game rules so
/// they can decide behaviour without reaching into the engine's internals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemInfo {
    pub id: EntityId,
    pub name: String,
    pub aliases: Vec<String>,
}

impl ItemInfo {
    pub(crate) fn from_item(item: &Item) -> Self {
        ItemInfo {
            id: item.id,
            name: item.primary_name.clone(),
            aliases: item
                .aliases
                .split(';')
                .filter(|a| !a.is_empty())
                .map(str::to_string)
                .collect(),
        }
    }
}
