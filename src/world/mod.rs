mod inventory;
mod item;
mod room;

use inventory::Inventory;
use room::Room;

pub(crate) use item::Item;

use crate::engine::EntityId;

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

    pub(crate) fn get_item_name(&self, id: EntityId) -> Option<String> {
        self.get_available_items()
            .iter()
            .find(|item| item.id == id)
            .map(|item| item.primary_name.clone())
    }

    pub(crate) fn is_item_in_room(&self, id: EntityId) -> bool {
        self.current_room.items.iter().any(|item| item.id == id)
    }

    pub(crate) fn add_item_to_room(
        &mut self,
        id: EntityId,
        primary_name: &str,
        aliases: Vec<&str>,
    ) {
        let item = Item {
            id,
            primary_name: primary_name.to_string(),
            aliases: aliases.into_iter().map(str::to_string).collect(),
        };
        self.current_room.items.push(item);
    }

    pub(crate) fn is_item_in_inventory(&self, id: EntityId) -> bool {
        self.player_inventory.items.iter().any(|item| item.id == id)
    }

    pub(crate) fn add_item_in_inventory(
        &mut self,
        id: EntityId,
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

    pub(crate) fn move_to_inventory(&mut self, id: EntityId) -> bool {
        if self.is_item_in_inventory(id) {
            return false;
        }

        if let Some(index) = self
            .current_room
            .items
            .iter()
            .position(|item| item.id == id)
        {
            let item = self.current_room.items.remove(index);
            self.player_inventory.items.push(item);
            true
        } else {
            false
        }
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

    pub(crate) fn handle_resolution_failure(
        &self,
        resolution: &Resolution,
        item: &str,
    ) -> ActionResult {
        match resolution {
            Resolution::Ambiguous(_) => {
                ActionResult::Failed(format!("Which {item} do you mean? Be more specific."))
            }
            Resolution::NotFound => ActionResult::Failed(format!("You don't see any {item} here.")),
            Resolution::Found(_) => {
                unreachable!("Found resolution should not reach failure handling")
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Resolution {
    Found(EntityId),
    Ambiguous(Vec<EntityId>),
    NotFound,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ActionResult {
    Success(String),
    Failed(String),
}

#[cfg(test)]
mod component_tests {
    use rstest::rstest;

    use super::*;

    // Helper fixture setup for world items
    fn setup_game() -> WorldState {
        let mut world = WorldState::new();
        // Adds items with an ID, primary name, and optional aliases
        world.add_item_to_room(1, "iron key", vec!["key"]);
        world.add_item_to_room(
            2,
            "glowing mysterious sword",
            vec!["glowing sword", "sword"],
        );
        world.add_item_to_room(3, "locked chest", vec!["chest"]);
        world.add_item_in_inventory(4, "brass key", vec!["key"]);
        world
    }

    #[rstest]
    #[case::exact_full_name("glowing mysterious sword", Resolution::Found(2))]
    #[case::partial_alias_match("glowing sword", Resolution::Found(2))]
    #[case::alias_match("iron key", Resolution::Found(1))]
    #[case::ambiguous_key("key", Resolution::Ambiguous(vec![1, 4]))]
    #[case::not_found("health potion", Resolution::NotFound)]
    fn resolves_entities_in_world(#[case] target: &str, #[case] expected: Resolution) {
        let world = setup_game();
        let result = world.resolve_entity(target);
        assert_eq!(expected, result);
    }

    #[test]
    fn test_take_item_success() {
        let mut world = setup_game();
        let result = world.move_to_inventory(1);

        assert!(result);
        assert!(!world.is_item_in_room(1));
        assert!(world.is_item_in_inventory(1));
    }

    #[test]
    fn test_take_item_already_in_inventory() {
        let mut world = setup_game();
        world.move_to_inventory(1);

        let result = world.move_to_inventory(1);

        assert!(!result);
    }

    #[test]
    fn test_handle_ambiguous_entity_resolution() {
        let world = setup_game();

        let resolution = Resolution::Ambiguous(vec![1, 2]);
        let result = world.handle_resolution_failure(&resolution, "key");

        assert_eq!(
            result,
            ActionResult::Failed("Which key do you mean? Be more specific.".to_string())
        );
    }

    #[test]
    fn test_handle_not_found_entity_resolution() {
        let world = setup_game();

        let resolution = Resolution::NotFound;
        let result = world.handle_resolution_failure(&resolution, "dragon");

        assert_eq!(
            result,
            ActionResult::Failed("You don't see any dragon here.".to_string())
        );
    }
}
