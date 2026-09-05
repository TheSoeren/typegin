#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ItemId(i32);

impl ItemId {
    pub fn new(value: i32) -> Self {
        ItemId(value)
    }

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

#[derive(Debug, PartialEq, Eq)]
pub enum ItemResolution {
    Found(ItemId),
    Ambiguous { ids: Vec<ItemId>, alias: String },
    NotFound,
}

use crate::data;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub(crate) id: ItemId,
    pub(crate) primary_name: String,
    pub(crate) aliases: String,
    pub(crate) extra: HashMap<String, data::ExtraValue>,
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
            _ => ItemResolution::Ambiguous {
                ids: matching_ids,
                alias: name.to_string(),
            },
        }
    }

    pub(crate) fn from_data(item: &data::ItemData) -> Self {
        Item {
            id: item.id.into(),
            primary_name: item.primary_name.clone(),
            aliases: item.aliases.join(";"),
            extra: item.extra.clone(),
        }
    }
}

/// Public, plain-data view of an item in the world, handed to game rules so
/// they can decide behaviour without reaching into the engine's internals.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemInfo {
    pub id: ItemId,
    pub name: String,
    pub aliases: Vec<String>,
    pub extra: HashMap<String, data::ExtraValue>,
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
            extra: item.extra.clone(),
        }
    }
}
