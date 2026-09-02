use std::error::Error;

use diesel::Connection;
use diesel::result::Error as DieselError;
use diesel::sqlite::SqliteConnection;
use diesel_migrations::MigrationHarness;

use crate::data::WorldData;
use crate::event::Event;
use crate::input::{Action, Direction, action, parse_input};
use crate::migrations::MIGRATIONS;
use crate::world;
use crate::world::item;

/// Numeric identifier for a room or other world entity.
///
/// Comparisons like `world.current_room_id() == RoomId::new(2)` and values in
/// `Action` resolution should be read as opaque ids backed by the database,
/// not as meaningful numbers on their own. For genuine room identities prefer
/// the type-safe [`RoomId`](crate::RoomId).
pub type EntityId = i32;

/// The pure game state + persistence. Holds no rendering or I/O logic.
///
/// Create one with [`GameEngine::open`] (stock rules) or
/// [`GameEngine::open_with_rules`] (custom [`Rules`]); then feed it raw text
/// via [`GameEngine::handle_input`] and let a [`View`] render the resulting
/// [`Event`]s.
///
/// Customization is done by injecting a [`Rules`] object at construction time
/// (via [`GameEngine::open_with_rules`]); there is no need to wrap the engine
/// in a newtype or re-delegate methods.
///
/// [`View`]: crate::view::View
pub struct GameEngine {
    pub world: world::WorldState,
    rules: Box<dyn Rules>,
    conn: SqliteConnection,
}

impl GameEngine {
    /// Open the engine at `db_path`, seeding and using the world defined by `data`.
    ///
    /// Uses the stock [`BasicRules`]; see [`GameEngine::open_with_rules`] to
    /// supply a custom [`Rules`] implementation.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be opened or migrated, or if
    /// the world cannot be loaded/created (e.g. `data` has no rooms).
    pub fn open(db_path: &str, data: &WorldData) -> Result<Self, Box<dyn Error + Send + Sync>> {
        Self::open_with_rules(db_path, data, BasicRules)
    }

    /// Open the engine at `db_path` with a custom [`Rules`] implementation.
    ///
    /// This is the entry point for customising behaviour: any rules that
    /// inherit from the defaults only need to override the hooks they care
    /// about (e.g. a `TakeRules` that moves items into the player's inventory).
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be opened or migrated, or if
    /// the world cannot be loaded/created (e.g. `data` has no rooms).
    pub fn open_with_rules(
        db_path: &str,
        data: &WorldData,
        rules: impl Rules + 'static,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let mut conn = SqliteConnection::establish(db_path)?;
        conn.run_pending_migrations(MIGRATIONS)?;

        let world = world::WorldState::load_or_seed(&mut conn, data)?;

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
        let world = world::WorldState::load_or_seed(&mut conn, data)?;

        Ok(GameEngine {
            world,
            rules: Box::new(rules),
            conn,
        })
    }

    /// Execute a raw textual command. Parses it, then runs `execute_action`.
    ///
    /// This is the typical per-turn entry point for a front-end: input arrives
    /// as a string, the engine parses it into an [`Action`] and applies it, and
    /// the returned [`Event`]s describe what happened so the UI can render them.
    pub fn handle_input(&mut self, input: &str) -> Vec<Event> {
        self.execute_action(parse_input(input))
    }

    /// Execute one parsed `Action` against the world, returning events.
    ///
    /// Parsing (e.g. with [`crate::parse_input`]) is separate from execution,
    /// so a front-end can reuse the same [`Action`] value multiple times or
    /// build one programmatically without going through text.
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
                let target_res = match target {
                    Some(ref r) => self.world.resolve_any_item(r),
                    None => item::ItemResolution::NotFound,
                };
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
    fn on_look(&mut self, _world: &world::WorldState) -> Vec<Event> {
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
    fn on_go(&mut self, world: &mut world::WorldState, direction: Direction) -> Vec<Event> {
        match world.get_room_id_by_exit_direction(direction) {
            Some(room_id) => {
                world.move_to_room(room_id);
                vec![Event::Went(direction)]
            }
            None => vec![Event::WentInvalidDirection(direction)],
        }
    }

    /// Decide what happens when the player tries to take an item.
    ///
    /// `resolution` is the outcome of matching `name` against the rooms items.
    fn on_take(
        &mut self,
        world: &mut world::WorldState,
        name: &str,
        resolution: item::ItemResolution,
    ) -> Vec<Event> {
        match resolution {
            item::ItemResolution::Found(id) => match world.player_take_item(id) {
                action::TakeResult::Success => vec![Event::Took {
                    item: name.to_string(),
                }],
                action::TakeResult::Fail => {
                    vec![Event::TookItemNotFound {
                        item: name.to_string(),
                    }]
                }
            },
            item::ItemResolution::Ambiguous(_) => vec![Event::TookItemAmbiguous {
                item: name.to_string(),
            }],
            item::ItemResolution::NotFound => vec![Event::TookItemNotFound {
                item: name.to_string(),
            }],
        }
    }

    /// Decide what happens when the player tries to drop an item.
    ///
    /// `resolution` is the outcome of matching `name` against the players items.
    fn on_drop(
        &mut self,
        world: &mut world::WorldState,
        name: &str,
        resolution: item::ItemResolution,
    ) -> Vec<Event> {
        match resolution {
            item::ItemResolution::Found(id) => match world.player_drop_item(id) {
                action::DropResult::Success => vec![Event::Dropped {
                    item: name.to_string(),
                }],
                action::DropResult::Fail => {
                    vec![Event::DroppedItemNotFound {
                        item: name.to_string(),
                    }]
                }
            },
            item::ItemResolution::Ambiguous(_) => vec![Event::DroppedItemAmbiguous {
                item: name.to_string(),
            }],
            item::ItemResolution::NotFound => vec![Event::DroppedItemNotFound {
                item: name.to_string(),
            }],
        }
    }

    /// Decide what happens when the player examines a thing.
    fn on_examine(
        &mut self,
        _world: &world::WorldState,
        name: &str,
        resolution: item::ItemResolution,
    ) -> Vec<Event> {
        match resolution {
            item::ItemResolution::Found(_) => {
                vec![Event::Examined {
                    item: name.to_string(),
                }]
            }
            item::ItemResolution::Ambiguous(_) => {
                vec![Event::ExaminedItemAmbiguous {
                    item: name.to_string(),
                }]
            }
            item::ItemResolution::NotFound => {
                vec![Event::ExaminedItemNotFound {
                    item: name.to_string(),
                }]
            }
        }
    }

    /// Decide what happens when the player uses an item on a target.
    fn on_use(
        &mut self,
        item: &str,
        target: Option<&str>,
        item_resolution: item::ItemResolution,
        target_resolution: item::ItemResolution,
    ) -> Vec<Event> {
        match item_resolution {
            item::ItemResolution::Found(_) => match target_resolution {
                item::ItemResolution::Found(_) => {
                    vec![Event::Used {
                        item: item.to_string(),
                        target: target.map(String::from),
                    }]
                }
                item::ItemResolution::Ambiguous(_) => {
                    vec![Event::UsedItemAmbiguous {
                        item: item.to_string(),
                    }]
                }
                item::ItemResolution::NotFound => {
                    vec![Event::UsedTargetNeeded {
                        item: item.to_string(),
                    }]
                }
            },
            item::ItemResolution::Ambiguous(_) => {
                vec![Event::UsedItemAmbiguous {
                    item: item.to_string(),
                }]
            }
            item::ItemResolution::NotFound => {
                vec![Event::UsedItemNotFound {
                    item: item.to_string(),
                }]
            }
        }
    }

    /// Decide what happens for an unrecognised command.
    fn on_unknown(&mut self, phrase: String) -> Vec<Event> {
        vec![Event::UnknownEvent { name: phrase }]
    }
}

/// Minimal rules that reuse every default hook.
///
/// Used when no custom rules are supplied to [`GameEngine::open`].
pub struct BasicRules;

impl Rules for BasicRules {}
