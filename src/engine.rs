use crate::{Action, Resolution, WorldState, parse, tokenize, world::ActionResult};

pub type EntityId = i32;

#[derive(Debug)]
pub struct GameEngine {
    pub(crate) world: WorldState,
}

impl GameEngine {
    pub(crate) fn new() -> Self {
        let world_state = WorldState::new();

        GameEngine { world: world_state }
    }

    pub(crate) fn handle_input(&mut self, input: &str) -> String {
        let tokens = tokenize(input);
        let token_refs: Vec<&str> = tokens.iter().map(String::as_str).collect();
        let action = parse(&token_refs);

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
        let resolution = self.world.resolve_entity(&input);
        match resolution {
            Resolution::Found(id) => {
                if self.world.is_item_in_inventory(id) {
                    return "You are already holding that.".to_string();
                }

                if self.world.move_to_inventory(id) {
                    let name = self.world.get_item_name(id);
                    format!("You take the {}.", name.unwrap_or_default())
                } else {
                    "Not in room.".to_string()
                }
            }
            _ => match self.world.handle_resolution_failure(&resolution, &input) {
                ActionResult::Success(message) => message,
                ActionResult::Failed(message) => message,
            },
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use crate::engine::GameEngine;

    fn setup_integration_game() -> GameEngine {
        let mut engine = GameEngine::new();
        // Setup room items: Entity 1 ("glowing mysterious sword")
        engine.world.add_item_to_room(
            1,
            "glowing mysterious sword",
            vec!["glowing sword", "sword"],
        );
        engine
    }

    #[test]
    fn test_full_pipeline_success() {
        let mut engine = setup_integration_game();

        let response = engine.handle_input("  TAKE the glowing, mysterious   sword! ");

        assert_eq!(response, "You take the glowing mysterious sword.");
        assert!(engine.world.is_item_in_inventory(1));
    }

    #[test]
    fn test_full_pipeline_unknown_command() {
        let mut engine = setup_integration_game();

        let response = engine.handle_input("dance wildly");

        assert_eq!(response, "I don't understand how to dance.");
    }

    #[test]
    fn test_full_pipeline_empty_command() {
        let mut engine = setup_integration_game();

        let response = engine.handle_input("");

        assert_eq!(response, "I don't understand how to do that.");
    }

    #[test]
    fn test_full_pipeline_missing_item() {
        let mut engine = setup_integration_game();

        let response = engine.handle_input("take ghost armor");

        assert_eq!(response, "You don't see any ghost armor here.");
    }
}
