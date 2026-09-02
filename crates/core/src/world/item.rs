use diesel::AsExpression;
use diesel::backend::Backend;
use diesel::deserialize;
use diesel::prelude::{Queryable, Selectable};
use diesel::serialize;
use diesel::sql_types::Integer;
use diesel::sqlite::Sqlite;

use crate::schema::items;

#[derive(Debug, Clone, Copy, PartialEq, Eq, AsExpression, deserialize::FromSqlRow)]
#[diesel(sql_type = Integer)]
pub struct ItemId(i32);

impl ItemId {
    /// Build an `ItemId` from its raw database value.
    pub fn new(value: i32) -> Self {
        ItemId(value)
    }

    /// The raw database value backing this id.
    pub fn get(self) -> i32 {
        self.0
    }
}

impl From<i32> for ItemId {
    fn from(value: i32) -> Self {
        ItemId::new(value)
    }
}

impl From<ItemId> for i32 {
    fn from(id: ItemId) -> Self {
        id.get()
    }
}

impl deserialize::FromSql<Integer, Sqlite> for ItemId {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let val = i32::from_sql(bytes)?;
        Ok(ItemId(val))
    }
}

// 2. Convert from ItemId -> DB (Sqlite)
impl serialize::ToSql<Integer, Sqlite> for ItemId {
    fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, Sqlite>) -> serialize::Result {
        serialize::ToSql::<Integer, Sqlite>::to_sql(&self.0, out)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ItemResolution {
    Found(ItemId),
    Ambiguous(Vec<ItemId>),
    NotFound,
}

#[derive(Debug, Clone, Queryable, Selectable, PartialEq, Eq)]
#[diesel(table_name = items)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Item {
    pub(crate) id: ItemId,
    pub(crate) primary_name: String,
    pub(crate) aliases: String,
}

impl Item {
    pub fn has_name(&self, name: &str) -> bool {
        self.primary_name == name || self.aliases.split(';').any(|alias| alias == name)
    }

    pub fn resolve_item_by_name(items: &[Item], name: &str) -> ItemResolution {
        let matching_ids: Vec<ItemId> = items
            .iter()
            .filter(|item| item.has_name(name))
            .map(|item| item.id)
            .collect();

        match matching_ids.len() {
            0 => ItemResolution::NotFound,
            1 => ItemResolution::Found(matching_ids[0]),
            _ => ItemResolution::Ambiguous(matching_ids),
        }
    }
}

/// Public, plain-data view of an item in the world, handed to game rules so
/// they can decide behaviour without reaching into the engine's internals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemInfo {
    pub id: ItemId,
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
