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
