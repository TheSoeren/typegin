use getset::Getters;

use crate::world::item;

#[derive(Debug, Getters)]
#[getset(get = "pub(crate)")]
pub(crate) struct Player {
    items: Vec<item::Item>,
}

impl Player {
    pub(crate) fn new() -> Self {
        Player { items: Vec::new() }
    }

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

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}
