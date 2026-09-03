use getset::{Getters, MutGetters};

use crate::data::WorldData;
use crate::event::Event;
use crate::input::{Action, parse_input};
use crate::rules::BasicRules;
use crate::rules::Rules;
use crate::world;
use crate::world::item;

/// The pure game state. Holds no rendering, I/O, or persistence logic.
///
/// Create one with [`GameEngine::open`] (stock rules) or
/// [`GameEngine::get_with_rules`] (custom [`Rules`]); then feed it raw text
/// via [`GameEngine::handle_input`] and let a [`View`] render the resulting
/// [`Event`]s.
///
/// Customization is done by injecting a [`Rules`] object at construction time
/// (via [`GameEngine::get_with_rules`]); there is no need to wrap the engine
/// in a newtype or re-delegate methods.
///
/// [`View`]: crate::view::View
#[derive(Getters, MutGetters)]
pub struct GameEngine {
    #[getset(get = "pub", get_mut = "pub")]
    world: world::WorldState,
    rules: Box<dyn Rules>,
}

impl GameEngine {
    /// Open the engine with the world defined by `data`.
    /// Uses the stock [`BasicRules`]
    pub fn get(data: &WorldData) -> Self {
        Self::get_with_rules(data, BasicRules)
    }

    /// Open the engine with the world defined by `data` and
    /// a custom [`Rules`] implementation.
    pub fn get_with_rules(data: &WorldData, rules: impl Rules + 'static) -> Self {
        let world = world::WorldState::from_data(data);

        GameEngine {
            world,
            rules: Box::new(rules),
        }
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
            Action::Look => self.rules.on_look(&mut self.world),
            Action::Go(direction) => self.rules.on_go(&mut self.world, direction),
            Action::Examine(name) => {
                let resolution = self.world.resolve_any_item(&name);
                self.rules.on_examine(&mut self.world, &name, resolution)
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
                self.rules.on_use(
                    &mut self.world,
                    &item,
                    target.as_deref(),
                    item_res,
                    target_res,
                )
            }
            Action::Unknown(phrase) => self.rules.on_unknown(&mut self.world, phrase),
        }
    }
}
