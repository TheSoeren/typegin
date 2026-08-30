mod item;
mod player;
mod room;

use diesel::prelude::*;
use diesel::result::Error as DieselError;
use diesel::sqlite::SqliteConnection;
use getset::Getters;
use room::Room;

use crate::EntityId;
use crate::data::{WorldData, load_world_data};
use crate::schema::{inventories, inventory_items, items, players, rooms, world_states};
use crate::world::item::Item;
use crate::world::player::Player;

#[derive(Debug, Getters)]
#[getset(get = "pub(crate)")]
pub struct WorldState {
    player: Player,
    current_room: Room,
}

impl WorldState {
    pub(crate) fn load_or_seed(conn: &mut SqliteConnection) -> Result<Self, DieselError> {
        let data = load_world_data();

        let count: i64 = world_states::table.count().get_result(conn)?;
        let world_id = if count == 0 {
            Self::seed(conn, &data)?
        } else {
            world_states::table.select(world_states::id).first(conn)?
        };

        Self::load(conn, world_id)
    }

    pub(crate) fn seed(
        conn: &mut SqliteConnection,
        data: &WorldData,
    ) -> Result<EntityId, DieselError> {
        let first_room_id = data
            .rooms
            .first()
            .expect("world data must contain at least one room")
            .id;

        for room in &data.rooms {
            let inventory_id: EntityId = diesel::insert_into(inventories::table)
                .default_values()
                .returning(inventories::id)
                .get_result(conn)?;

            diesel::insert_into(rooms::table)
                .values((rooms::id.eq(room.id), rooms::inventory_id.eq(inventory_id)))
                .execute(conn)?;

            for item_id in &room.visible_items {
                diesel::insert_into(inventory_items::table)
                    .values((
                        inventory_items::inventory_id.eq(inventory_id),
                        inventory_items::item_id.eq(item_id),
                        inventory_items::hidden.eq(false),
                    ))
                    .execute(conn)?;
            }

            for item_id in &room.hidden_items {
                diesel::insert_into(inventory_items::table)
                    .values((
                        inventory_items::inventory_id.eq(inventory_id),
                        inventory_items::item_id.eq(item_id),
                        inventory_items::hidden.eq(true),
                    ))
                    .execute(conn)?;
            }
        }

        for item in &data.items {
            diesel::insert_into(items::table)
                .values((
                    items::id.eq(item.id),
                    items::primary_name.eq(&item.primary_name),
                    items::aliases.eq(item.aliases.join(";")),
                ))
                .execute(conn)?;
        }

        let player_inventory_id: EntityId = diesel::insert_into(inventories::table)
            .default_values()
            .returning(inventories::id)
            .get_result(conn)?;

        let player_id: EntityId = diesel::insert_into(players::table)
            .values(players::inventory_id.eq(player_inventory_id))
            .returning(players::id)
            .get_result(conn)?;

        diesel::insert_into(world_states::table)
            .values((
                world_states::player_id.eq(player_id),
                world_states::current_room_id.eq(first_room_id),
            ))
            .returning(world_states::id)
            .get_result(conn)
    }

    pub(crate) fn load(conn: &mut SqliteConnection, id: EntityId) -> Result<Self, DieselError> {
        let (player_id, current_room_id): (EntityId, EntityId) = world_states::table
            .find(id)
            .select((world_states::player_id, world_states::current_room_id))
            .first(conn)?;

        let player = Player::load(conn, player_id)?;
        let current_room = Room::load(conn, current_room_id)?;

        Ok(WorldState {
            player,
            current_room,
        })
    }

    pub fn get_item_name(&self, id: EntityId) -> Option<String> {
        self.get_available_items()
            .iter()
            .find(|item| item.id == id)
            .map(|item| item.primary_name.clone())
    }

    pub fn move_item_to_inventory(&mut self, id: EntityId) -> bool {
        if self.player.has_item(id) {
            return false;
        }

        match self.current_room.remove_item(id) {
            Some(item) => {
                self.player.add_item(item);
                true
            }
            None => false,
        }
    }

    pub fn resolve_entity(&self, name: &str) -> Resolution {
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
            self.current_room().items().as_slice(),
            self.player().items().as_slice(),
        ]
        .concat()
    }

    pub fn handle_resolution_failure(&self, resolution: &Resolution, item: &str) -> ActionResult {
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
    use crate::data::load_world_data;
    use crate::test_db::test_connection;

    fn setup_game() -> WorldState {
        let mut conn = test_connection();

        WorldState::seed(&mut conn, &load_world_data())
            .and_then(|world_id| WorldState::load(&mut conn, world_id))
            .expect("seed and load world")
    }

    #[rstest]
    #[case::exact_full_name("glowing mysterious sword", Resolution::Found(1))]
    #[case::partial_alias_match("glowing sword", Resolution::Found(1))]
    #[case::alias_match("iron key", Resolution::Found(2))]
    #[case::ambiguous_key("key", Resolution::Ambiguous(vec![2, 4]))]
    #[case::not_found("health potion", Resolution::NotFound)]
    fn resolves_entities_in_world(#[case] target: &str, #[case] expected: Resolution) {
        let world = setup_game();
        let result = world.resolve_entity(target);
        assert_eq!(expected, result);
    }

    #[test]
    fn test_take_item_success() {
        let mut world = setup_game();
        let result = world.move_item_to_inventory(2);

        assert!(result);
        assert!(!world.current_room.has_item(2));
        assert!(world.player.has_item(2));
    }

    #[test]
    fn test_take_item_already_in_inventory() {
        let mut world = setup_game();
        world.move_item_to_inventory(2);

        let result = world.move_item_to_inventory(2);

        assert!(!result);
    }

    #[test]
    fn test_seed_populates_inventories() {
        let mut conn = test_connection();

        let world_id = WorldState::seed(&mut conn, &load_world_data()).expect("seed world");

        let loaded = WorldState::load(&mut conn, world_id).expect("load world");

        assert!(loaded.current_room.has_item(1));
        assert!(loaded.current_room.has_item(2));
        assert!(!loaded.current_room.has_item(5));
        assert!(loaded.player.items().is_empty());
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
