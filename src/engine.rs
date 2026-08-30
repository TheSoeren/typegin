use diesel::result::Error as DieselError;
use diesel::sqlite::SqliteConnection;
use input_parser::parse_input;

use crate::{Action, Resolution, WorldState, world::ActionResult};

pub type EntityId = i32;

pub struct GameEngine {
    pub(crate) world: WorldState,
    conn: SqliteConnection,
}

impl GameEngine {
    pub(crate) fn new(mut conn: SqliteConnection) -> Result<Self, DieselError> {
        let world = WorldState::load_or_seed(&mut conn)?;

        Ok(GameEngine { world, conn })
    }

    pub(crate) fn load(conn: SqliteConnection, world_id: EntityId) -> Result<Self, DieselError> {
        let mut conn = conn;
        let world = WorldState::load(&mut conn, world_id)?;

        Ok(GameEngine { world, conn })
    }

    pub(crate) fn handle_input(&mut self, input: &str) -> String {
        let action = parse_input(input);

        match action {
            Action::Look => "You look around.".to_string(),
            Action::Go(direction) => format!("You go {direction:?}."),
            Action::Examine(name) => format!("You examine the {name}."),
            Action::Take(name) => self.execute_take(name),
            Action::Use { item, target } => match target {
                Some(target) => format!("You use the {item} on the {target}."),
                None => format!("You use the {item}."),
            },
            Action::Unknown(name) => format!("I don't understand how to {name}."),
        }
    }

    fn execute_take(&mut self, input: String) -> String {
        match self.world.resolve_entity(&input) {
            Resolution::Found(id) => {
                if self.world.player().has_item(id) {
                    return "You are already holding that.".to_string();
                }

                if self.world.move_item_to_inventory(id) {
                    let name = self.world.get_item_name(id);
                    format!("You take the {}.", name.unwrap_or_default())
                } else {
                    "Not in room.".to_string()
                }
            }
            resolution => match self.world.handle_resolution_failure(&resolution, &input) {
                ActionResult::Success(message) => message,
                ActionResult::Failed(message) => message,
            },
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use crate::engine::GameEngine;
    use crate::test_db::test_connection;

    fn setup_integration_game() -> GameEngine {
        let conn = test_connection();
        GameEngine::new(conn).expect("create engine")
    }

    #[test]
    fn test_full_pipeline_success() {
        let mut engine = setup_integration_game();

        let response = engine.handle_input("  TAKE the glowing, mysterious   sword! ");

        assert_eq!(response, "You take the glowing mysterious sword.");
        assert!(engine.world.player().has_item(1));
        assert!(!engine.world.current_room().has_item(1));
    }

    #[test]
    fn test_full_pipeline_unknown_command() {
        let mut engine = setup_integration_game();

        let response = engine.handle_input("dance wildly");

        assert_eq!(response, "I don't understand how to dance wildly.");
    }

    #[test]
    fn test_full_pipeline_empty_command() {
        let mut engine = setup_integration_game();

        let response = engine.handle_input("");

        assert_eq!(response, "I don't understand how to .");
    }

    #[test]
    fn test_full_pipeline_missing_item() {
        let mut engine = setup_integration_game();

        let response = engine.handle_input("take ghost armor");

        assert_eq!(response, "You don't see any ghost armor here.");
    }
}
