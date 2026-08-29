mod inventory;
mod item;
mod room;

use inventory::Inventory;
use room::Room;

pub(crate) use item::Item;

#[derive(Debug)]
pub struct WorldState {
    player_inventory: Inventory,
    current_room: Room,
}

impl WorldState {
    pub(crate) fn new() -> Self {
        let inventory = Inventory { items: vec![] };
        let room = Room {
            id: 1,
            name: "Test Room".to_string(),
            items: vec![],
            hidden_items: vec![],
        };

        WorldState {
            player_inventory: inventory,
            current_room: room,
        }
    }

    pub(crate) fn add_item_to_room(&mut self, id: i32, primary_name: &str, aliases: Vec<&str>) {
        let item = Item {
            id,
            primary_name: primary_name.to_string(),
            aliases: aliases.into_iter().map(str::to_string).collect(),
        };
        self.current_room.items.push(item);
    }

    pub(crate) fn add_item_in_inventory(
        &mut self,
        id: i32,
        primary_name: &str,
        aliases: Vec<&str>,
    ) {
        let item = Item {
            id,
            primary_name: primary_name.to_string(),
            aliases: aliases.into_iter().map(str::to_string).collect(),
        };
        self.player_inventory.items.push(item);
    }

    pub(crate) fn resolve_entity(&self, name: &str) -> Resolution {
        let matching_ids: Vec<EntityId> = self
            .get_available_items()
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
            self.current_room.items.as_slice(),
            self.player_inventory.items.as_slice(),
        ]
        .concat()
    }
}

pub type EntityId = i32;

#[derive(Debug, PartialEq, Eq)]
pub enum Resolution {
    Found(EntityId),
    Ambiguous(Vec<EntityId>),
    NotFound,
}
