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
            Action::Go(direction) => self.rules.on_go(&mut self.world, direction),
            Action::Examine(name) => {
                let resolution = self.world.resolve_any_item(&name);
                self.rules.on_examine(&self.world, &name, resolution)
            }
            Action::Take(name) => {
                let resolution = self.world.resolve_room_item(&name);
                self.rules.on_take(&mut self.world, &name, resolution)
            }
            Action::Drop(name) => {
                let resolution = self.world.resolve_player_item(&name);
                self.rules.on_drop(&mut self.world, &name, resolution)
            }
            Action::Use { item, target } => {
                let item_res = self.world.resolve_player_item(&item);
                let target_res = target
                    .as_deref()
                    .map(|t| self.world.resolve_any_item(t))
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
    ///
    /// The world is mutable so a rule has full control: it may call
    /// [`WorldState::get_room_id_by_exit_direction`] to resolve the target room and
    /// [`WorldState::move_to_room`] to actually change rooms — or deliberately
    /// not move, to block an otherwise valid exit (e.g. a locked door).
    ///
    /// The default follows the exit, emitting nothing if there is none.
    fn on_go(&mut self, world: &mut WorldState, direction: Direction) -> Vec<Event> {
        match world.get_room_id_by_exit_direction(direction) {
            Some(room_id) => {
                world.move_to_room(room_id);
                vec![Event::Went(direction)]
            }
            None => vec![Event::Message(format!("To the {} is no exit", direction))],
        }
    }

    /// Decide what happens when the player tries to take an item.
    ///
    /// `resolution` is the outcome of matching `name` against the rooms items.
    fn on_take(&mut self, world: &mut WorldState, name: &str, resolution: Resolution)
    -> Vec<Event> {
        let response_events: &mut Vec<Event> = &mut vec![];
        match resolution {
            Resolution::Found(id) => {
                if world.player_has_item(id) {
                    response_events.push(Event::AlreadyHolding {
                        item: name.to_string(),
                    });
                    todo!("Implement behavior when player already has item -> item count +1");
                }

                if world.move_item_to_inventory(id) {
                    let item = world
                        .item_info(id)
                        .map(|info| info.name)
                        .unwrap_or_else(|| name.to_string());
                    vec![Event::Took { item }]
                } else {
                    response_events.push(Event::NotFound {
                        phrase: name.to_string(),
                    });
                    unreachable!(
                        "on_take must only be called if the item is known to be in the room!"
                    );
                }
            }
            Resolution::Ambiguous(_) => vec![Event::Ambiguous {
                phrase: name.to_string(),
            }],
            Resolution::NotFound => vec![Event::NotFound {
                phrase: name.to_string(),
            }],
        }
    }

    /// Decide what happens when the player tries to drop an item.
    ///
    /// `resolution` is the outcome of matching `name` against the players items.
    fn on_drop(&mut self, world: &mut WorldState, name: &str, resolution: Resolution)
    -> Vec<Event> {
        let response_events: &mut Vec<Event> = &mut vec![];
        match resolution {
            Resolution::Found(id) => {
                if world.room_has_item(id) {
                    response_events.push(Event::AlreadyHolding {
                        item: name.to_string(),
                    });
                    todo!("Implement behavior when player already has item -> item count +1");
                }

                if world.move_item_from_inventory(id) {
                    let item = world
                        .item_info(id)
                        .map(|info| info.name)
                        .unwrap_or_else(|| name.to_string());
                    vec![Event::Dropped { item }]
                } else {
                    response_events.push(Event::NotFound {
                        phrase: name.to_string(),
                    });
                    unreachable!(
                        "on_drop must only be called if the item is known to be in the room!"
                    );
                }
            }
            Resolution::Ambiguous(_) => vec![Event::Ambiguous {
                phrase: name.to_string(),
            }],
            Resolution::NotFound => vec![Event::NotFound {
                phrase: name.to_string(),
            }],
        }
    }

    /// Decide what happens when the player examines a thing.
    fn on_examine(&mut self, _world: &WorldState, name: &str, resolution: Resolution) -> Vec<Event> {
        match resolution {
            Resolution::Found(_) => vec![Event::Message(format!("You examine the {name}."))],
            Resolution::Ambiguous(_) => {
                vec![Event::Message(format!(
                    "Be more specific, there are multiple {}",
                    name
                ))]
            }
            Resolution::NotFound => vec![Event::Message(format!("There is no {name}."))],
        }
    }

    /// Decide what happens when the player uses an item on a target.
    fn on_use(
        &mut self,
        item: &str,
        target: Option<&str>,
        item_resolution: Resolution,
        target_resolution: Resolution,
    ) -> Vec<Event> {
        match item_resolution {
            Resolution::Found(_) => {
                if let Some(t) = target
                    && target_resolution == Resolution::NotFound
                {
                    return vec![Event::Message(format!("You can't use that on {t}."))];
                }

                vec![Event::Used {
                    item: item.to_string(),
                    target: target.map(str::to_string),
                }]
            }
            Resolution::Ambiguous(_) => {
                vec![Event::Message(format!(
                    "Be more specific, there are multiple {item}."
                ))]
            }
            Resolution::NotFound => vec![Event::Message(format!("You don't have a {item}."))],
        }
    }

    /// Decide what happens for an unrecognised command.
    fn on_unknown(&mut self, phrase: String) -> Vec<Event> {
        vec![Event::Message(format!(
            "I don't understand how to \"{phrase}\"."
        ))]
    }
}

/// Minimal rules that reuse every default hook.
///
/// Used when no custom rules are supplied to [`GameEngine::open`].
pub struct BasicRules;

impl Rules for BasicRules {}
