use getset::{Getters, MutGetters};

use crate::data::WorldData;
use crate::event::Event;
use crate::input::{Action, parse_input};
use crate::interaction::{ActionContext, Interaction, Verb};
use crate::rules::BasicRules;
use crate::rules::Rules;
use crate::world;
use crate::world::object::{self, ObjectId};

/// The pure game state. Holds no rendering, I/O, or persistence logic.
///
/// Create one with [`GameEngine::get`] (stock rules) or
/// [`GameEngine::get_with_rules`] (custom [`Rules`]); then feed it raw text
/// via [`GameEngine::handle_input`] and let a [`View`] render the resulting
/// [`Event`]s.
///
/// Customization is done by injecting a [`Rules`] object at construction time
/// (via [`GameEngine::get_with_rules`]); there is no need to wrap the engine
/// in a newtype or re-delegate methods.
///
/// The engine is front-end agnostic by design: a text adventure feeds it the
/// same [`Action`]s that a GUI synthesizes from clicks, and a point-and-click
/// UI can *query* what is currently possible via
/// [`GameEngine::interactions_for`] instead of re-implementing puzzle logic.
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
                let resolution = self.world.resolve_target(&name);
                self.rules.on_examine(&mut self.world, &name, resolution)
            }
            Action::Take(name) => {
                let resolution = self.world.resolve_room_object(&name);
                self.rules.on_take(&mut self.world, &name, resolution)
            }
            Action::Drop(name) => {
                let resolution = self.world.resolve_player_object(&name);
                self.rules.on_drop(&mut self.world, &name, resolution)
            }
            Action::Use { item, target } => {
                let item_res = self.world.resolve_player_object(&item);
                let target_res = match target {
                    Some(ref name) => self.world.resolve_target(name),
                    None => object::ObjectResolution::NotFound,
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

    /// Query which authored interactions are currently live for a given
    /// context, without executing any of them.
    ///
    /// This is the point-and-click hook: a GUI asks "what can the player do
    /// with this target right now?" and gets back the interactions whose
    /// conditions hold. `item` is the object being held/used (if any),
    /// `target` the resolved target object's id.
    ///
    /// Stock `BasicRules` behaviour is not listed here — the query reports
    /// *authored* interactions only; a front-end combines the result with the
    /// world's own state (e.g. a locked door whose `gated_by` object is held)
    /// to decide what to offer.
    pub fn interactions_for(
        &self,
        item: Option<ObjectId>,
        target: Option<ObjectId>,
    ) -> Vec<&Interaction> {
        let context = ActionContext::new(item, target);
        self.rules
            .interactions()
            .iter()
            .filter(|interaction| {
                interaction.verb() == Verb::Use && interaction.matches(&self.world, &context)
            })
            .collect()
    }
}
