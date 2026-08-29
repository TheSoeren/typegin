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

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    // Helper fixture setup for world items
    fn sample_world() -> WorldState {
        let mut world = WorldState::new();
        // Adds items with an ID, primary name, and optional aliases
        world.add_item_to_room(
            1,
            "glowing mysterious sword",
            vec!["glowing sword", "sword"],
        );
        world.add_item_to_room(2, "heavy iron key", vec!["iron key", "key"]);
        world.add_item_in_inventory(3, "brass key", vec!["key"]);
        world
    }

    #[rstest]
    #[case::exact_full_name("glowing mysterious sword", Resolution::Found(1))]
    #[case::partial_alias_match("glowing sword", Resolution::Found(1))]
    #[case::alias_match("iron key", Resolution::Found(2))]
    #[case::ambiguous_key("key", Resolution::Ambiguous(vec![2, 3]))]
    #[case::not_found("health potion", Resolution::NotFound)]
    fn resolves_entities_in_world(#[case] target: &str, #[case] expected: Resolution) {
        let world = sample_world();
        let result = world.resolve_entity(target);
        assert_eq!(expected, result);
    }
}
