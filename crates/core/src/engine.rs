use std::error::Error;

use diesel::Connection;
use diesel::result::Error as DieselError;
use diesel::sqlite::SqliteConnection;
use diesel_migrations::MigrationHarness;

use crate::data::WorldData;
use crate::event::Event;
use crate::input::{Action, Direction, parse_input};
use crate::migrations::MIGRATIONS;
use crate::world::{Resolution, WorldState};

pub type EntityId = i32;

/// The pure game state + persistence. Holds no rendering or I/O logic.
///
/// Customization is done by injecting a [`Rules`] object at construction time
/// (via [`GameEngine::open_with_rules`]); there is no need to wrap the engine
/// in a newtype or re-delegate methods.
pub struct GameEngine {
    pub world: WorldState,
    rules: Box<dyn Rules>,
    conn: SqliteConnection,
}

impl GameEngine {
    pub fn open(db_path: &str, data: &WorldData) -> Result<Self, Box<dyn Error + Send + Sync>> {
        Self::open_with_rules(db_path, data, BasicRules)
    }

    pub fn open_with_rules(
        db_path: &str,
        data: &WorldData,
        rules: impl Rules + 'static,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let mut conn = SqliteConnection::establish(db_path)?;
        conn.run_pending_migrations(MIGRATIONS)?;

        let world = WorldState::load_or_seed(&mut conn, data)?;

        Ok(GameEngine {
            world,
            rules: Box::new(rules),
            conn,
        })
    }

    pub(crate) fn new(conn: SqliteConnection, data: &WorldData) -> Result<Self, DieselError> {
        Self::new_with_rules(conn, data, BasicRules)
    }

    pub(crate) fn new_with_rules(
        mut conn: SqliteConnection,
        data: &WorldData,
        rules: impl Rules + 'static,
    ) -> Result<Self, DieselError> {
        let world = WorldState::load_or_seed(&mut conn, data)?;

        Ok(GameEngine {
            world,
            rules: Box::new(rules),
            conn,
        })
    }

    /// Execute a raw textual command. Parses it, then runs `execute_action`.
    pub fn handle_input(&mut self, input: &str) -> Vec<Event> {
        self.execute_action(parse_input(input))
    }

    /// Execute one parsed `Action` against the world, returning events.
    pub fn execute_action(&mut self, action: Action) -> Vec<Event> {
        match action {
            Action::Look => self.rules.on_look(&self.world),
            Action::Go(direction) => self.rules.on_go(&self.world, direction),
            Action::Examine(name) => {
                let resolution = self.world.resolve_entity(&name);
                self.rules.on_examine(&self.world, &name, resolution)
            }
            Action::Take(name) => {
                let resolution = self.world.resolve_entity(&name);
                self.rules.on_take(&mut self.world, &name, resolution)
            }
            Action::Use { item, target } => {
                let item_res = self.world.resolve_entity(&item);
                let target_res = target
                    .as_deref()
                    .map(|t| self.world.resolve_entity(t))
                    .unwrap_or(Resolution::NotFound);
                self.rules
                    .on_use(&item, target.as_deref(), item_res, target_res)
            }
            Action::Unknown(phrase) => self.rules.on_unknown(phrase),
        }
    }
}

/// Hooks the game logic uses to decide behaviour.
///
/// This is how a game customizes rules: implement `Rules` and pass it to
/// [`GameEngine::open_with_rules`]. The world is passed in, so there is no
/// wrapper boilerplate and no delegation. Every method has a default
/// implementation, so a custom type only implements the hooks it wants to
/// change.
///
/// For actions that reference world entities (`take`, `examine`, `use`), the
/// engine resolves the name before calling the hook and passes both the
/// resulting [`Resolution`] and the ability to look up the matched
/// [`ItemInfo`] from the world — so a rule can act on the real entity (its
/// id, name and aliases) rather than a raw name string.
pub trait Rules {
    /// Decide what happens when the player looks around the room.
    fn on_look(&mut self, _world: &WorldState) -> Vec<Event> {
        vec![Event::Looked]
    }

    /// Decide what happens when the player moves in a direction.
    fn on_go(&mut self, _world: &WorldState, direction: Direction) -> Vec<Event> {
        vec![Event::Went(direction)]
    }

    /// Decide what happens when the player tries to take an item.
    ///
    /// `resolution` is the outcome of matching `name` against the world.
    fn on_take(
        &mut self,
        _world: &mut WorldState,
        name: &str,
        _resolution: Resolution,
    ) -> Vec<Event> {
        vec![Event::Took {
            item: name.to_string(),
        }]
    }

    /// Decide what happens when the player examines a thing.
    fn on_examine(
        &mut self,
        _world: &WorldState,
        name: &str,
        _resolution: Resolution,
    ) -> Vec<Event> {
        vec![Event::Message(format!("You examine the {name}."))]
    }

    /// Decide what happens when the player uses an item on a target.
    fn on_use(
        &mut self,
        item: &str,
        target: Option<&str>,
        _item_resolution: Resolution,
        _target_resolution: Resolution,
    ) -> Vec<Event> {
        vec![Event::Used {
            item: item.to_string(),
            target: target.map(str::to_string),
        }]
    }

    /// Decide what happens for an unrecognised command.
    fn on_unknown(&mut self, phrase: String) -> Vec<Event> {
        vec![Event::NotFound { phrase }]
    }
}

/// Minimal rules that reuse every default hook.
///
/// Used when no custom rules are supplied to [`GameEngine::open`].
pub struct BasicRules;

impl Rules for BasicRules {}

#[cfg(test)]
mod integration_tests {
    use crate::engine::{BasicRules, GameEngine, Rules};
    use crate::event::Event;
    use crate::input::Action;
    use crate::test_db::test_connection;
    use crate::world::Resolution;

    fn setup_integration_game() -> GameEngine {
        let conn = test_connection();
        GameEngine::new(conn, &crate::data::test_world_data()).expect("create engine")
    }

    struct StuckSwordRules;

    impl Rules for StuckSwordRules {
        fn on_take(
            &mut self,
            world: &mut super::WorldState,
            name: &str,
            resolution: Resolution,
        ) -> Vec<Event> {
            // The sword is identified by its id, not by string-matching.
            if matches!(resolution, Resolution::Found(1)) {
                vec![Event::Message(
                    "The sword is stuck in the stone.".to_string(),
                )]
            } else {
                BasicRules.on_take(world, name, resolution)
            }
        }
    }

    #[test]
    fn test_custom_rules_override() {
        let conn = test_connection();
        let mut engine =
            GameEngine::new_with_rules(conn, &crate::data::test_world_data(), StuckSwordRules)
                .expect("create engine");

        assert_eq!(
            engine.handle_input("take the glowing mysterious sword"),
            vec![Event::Message(
                "The sword is stuck in the stone.".to_string()
            )]
        );
        // The sword is stuck, so it must NOT have been taken.
        assert!(!engine.world.player_has_item(1));
        assert!(engine.world.room_has_item(1));

        assert_eq!(
            engine.handle_input("take the iron key"),
            vec![Event::Took {
                item: "iron key".to_string()
            }]
        );
        // Non-overridden commands fall through to the default `BasicRules`
        // hook, which emits the event but does not touch the inventory.
        assert!(!engine.world.player_has_item(2));
    }

    #[test]
    fn test_full_pipeline_take() {
        let mut engine = setup_integration_game();

        let events = engine.handle_input("  TAKE the glowing, mysterious   sword! ");

        assert_eq!(
            events,
            vec![Event::Took {
                item: "glowing mysterious sword".to_string()
            }]
        );
        // The default rules don't move items; only the pipeline is exercised.
        assert!(!engine.world.player_has_item(1));
    }

    #[test]
    fn test_rules_receive_entity_context() {
        let conn = test_connection();
        let mut engine =
            GameEngine::new_with_rules(conn, &crate::data::test_world_data(), InspectRules)
                .expect("create engine");

        let events = engine.handle_input("take the glowing mysterious sword");
        assert_eq!(
            events,
            vec![Event::Message(
                "context: id=1 aliases=[\"glowing sword\", \"sword\"]".to_string()
            )]
        );
    }

    struct InspectRules;

    impl Rules for InspectRules {
        fn on_take(
            &mut self,
            world: &mut super::WorldState,
            name: &str,
            resolution: Resolution,
        ) -> Vec<Event> {
            if let Resolution::Found(id) = resolution {
                if let Some(info) = world.item_info(id) {
                    return vec![Event::Message(format!(
                        "context: id={} aliases={:?}",
                        info.id, info.aliases
                    ))];
                }
            }
            let _ = name;
            vec![Event::Message("no context".to_string())]
        }
    }

    #[test]
    fn test_full_pipeline_unknown_command() {
        let mut engine = setup_integration_game();

        let events = engine.handle_input("dance wildly");

        assert_eq!(
            events,
            vec![Event::NotFound {
                phrase: "dance wildly".to_string()
            }]
        );
    }

    #[test]
    fn test_full_pipeline_empty_command() {
        let mut engine = setup_integration_game();

        let events = engine.handle_input("");

        assert_eq!(
            events,
            vec![Event::NotFound {
                phrase: String::new()
            }]
        );
    }

    #[test]
    fn test_execute_action_directly_without_parsing() {
        let mut engine = setup_integration_game();

        let events = engine.execute_action(Action::Take("iron key".to_string()));

        assert_eq!(
            events,
            vec![Event::Took {
                item: "iron key".to_string()
            }]
        );
    }

    #[test]
    fn test_engine_forwards_not_found_resolution() {
        let conn = test_connection();
        let mut engine =
            GameEngine::new_with_rules(conn, &crate::data::test_world_data(), EchoResolutionRules)
                .expect("create engine");

        let events = engine.handle_input("take ghost armor");

        assert_eq!(
            events,
            vec![Event::Message("ghost armor -> NotFound".to_string())]
        );
    }

    #[test]
    fn test_engine_forwards_ambiguous_resolution() {
        let conn = test_connection();
        let mut engine =
            GameEngine::new_with_rules(conn, &crate::data::test_world_data(), EchoResolutionRules)
                .expect("create engine");

        let events = engine.handle_input("take key");

        assert_eq!(
            events,
            vec![Event::Message("key -> Ambiguous([2, 4])".to_string())]
        );
    }

    /// Reports the `Resolution` the engine computed, so tests can verify the
    /// engine resolves names before calling the hook. It adds no game
    /// behaviour of its own.
    struct EchoResolutionRules;

    impl Rules for EchoResolutionRules {
        fn on_take(
            &mut self,
            _world: &mut super::WorldState,
            name: &str,
            resolution: Resolution,
        ) -> Vec<Event> {
            vec![Event::Message(format!("{name} -> {resolution:?}"))]
        }
    }
}
