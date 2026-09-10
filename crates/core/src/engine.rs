use getset::{Getters, MutGetters};

use crate::data::WorldData;
use crate::data::interactions_data::InteractionData;
use crate::event::Event;
use crate::input::{Action, parse_input};
use crate::interaction::{ActionContext, Interaction, Target};
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
    /// Data-driven interactions compiled into the closure [`Interaction`]
    /// shape, in declaration order. Consulted by `interactions_for` ahead of
    /// `Rules::interactions()`; dispatch itself goes through the default rules
    /// hooks (which read the raw data from the world).
    data_interactions: Vec<Interaction>,
    /// One synthetic [`Interaction::talk_npc`] hotspot per NPC in the world,
    /// so `interactions_for` reports current-room NPCs as [`Verb::Talk`]
    /// targets for a point-and-click front-end. Presence is a live condition
    /// (the NPC's room vs the player's current room), never a refresh.
    talk_targets: Vec<Interaction>,
}

impl GameEngine {
    /// Open the engine with the world defined by `data`.
    /// Uses the stock [`BasicRules`]
    #[must_use]
    pub fn get(data: &WorldData) -> Self {
        Self::get_with_rules(data, BasicRules)
    }

    /// Open the engine with the world defined by `data` and
    /// a custom [`Rules`] implementation.
    pub fn get_with_rules(data: &WorldData, rules: impl Rules + 'static) -> Self {
        let world = world::WorldState::from_data(data);
        let talk_targets = world
            .npcs()
            .iter()
            .map(|npc| Interaction::talk_npc(npc.id().clone()))
            .collect();

        GameEngine {
            world,
            rules: Box::new(rules),
            data_interactions: data
                .interactions
                .iter()
                .map(InteractionData::compile)
                .collect(),
            talk_targets,
        }
    }

    /// Execute a raw textual command. Parses it, then runs `execute_action`.
    ///
    /// This is the typical per-turn entry point for a front-end: input arrives
    /// as a string, the engine parses it into an [`Action`] and applies it, and
    /// the returned [`Event`]s describe what happened so the UI can render them.
    pub fn handle_input(&mut self, input: &str) -> Vec<Event> {
        let action = parse_input(input);
        self.execute_action(action)
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
                    None => object::TargetResolution::NotFound,
                };
                self.rules.on_use(
                    &mut self.world,
                    &item,
                    target.as_deref(),
                    item_res,
                    target_res,
                )
            }
            Action::Talk(name) => self.rules.on_talk(&mut self.world, &name),
            Action::Choose(choice) => self.rules.on_choose(&mut self.world, &choice),
            Action::Unknown(phrase) => self.rules.on_unknown(&mut self.world, phrase),
        }
    }

    /// Query which authored interactions are currently live for a given
    /// context, without executing any of them.
    #[must_use]
    pub fn interactions_for(
        &self,
        item: Option<ObjectId>,
        target: Option<Target>,
    ) -> Vec<&Interaction> {
        let context = ActionContext::new(None, item, target);
        // The open "what can I click" query is the only one NPC hotspots
        // belong in: they carry no item, so re-using their `matches` with an
        // item-constrained context would report them for every carried object.
        let open_query = context.item.is_none() && context.target.is_none();
        self.data_interactions
            .iter()
            .filter(|interaction| interaction.matches(&self.world, &context))
            .chain(
                self.rules
                    .interactions()
                    .iter()
                    .filter(|interaction| interaction.matches(&self.world, &context)),
            )
            .chain(
                open_query
                    .then_some(&self.talk_targets)
                    .into_iter()
                    .flatten()
                    .filter(|interaction| interaction.matches(&self.world, &context)),
            )
            .collect()
    }
}
