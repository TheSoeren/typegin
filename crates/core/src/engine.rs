use std::collections::HashSet;

use getset::{Getters, MutGetters};

use crate::data::WorldData;
use crate::data::interactions_data::InteractionData;
use crate::event::Event;
use crate::input::{Action, parse_input};
use crate::interaction::{ActionContext, Interaction, Target};
use crate::rules::BasicRules;
use crate::rules::Rules;
use crate::trigger::check_triggers;
use crate::world::SaveError;
use crate::world::object::{self, ObjectId};
use crate::{Verb, world};

/// The pure game state: no rendering, I/O, or persistence logic.
///
/// Create one with [`GameEngine::get`] or [`GameEngine::get_with_rules`] (to
/// inject custom [`Rules`]), feed it text via [`GameEngine::handle_input`],
/// and render the resulting [`Event`]s with a [`View`](crate::view::View). A
/// point-and-click front-end can instead query what is currently possible via
/// [`GameEngine::interactions_for`] and [`GameEngine::verbs_for`], without
/// executing anything.
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
    /// targets for a point-and-click front-end — both the open "what is
    /// clickable" query and a query targeted at that NPC specifically.
    /// Presence is a live condition (the NPC's room vs the player's current
    /// room), never a refresh.
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
        let mut events = match action {
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
        };
        events.extend(check_triggers(&mut self.world));
        events
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
                self.talk_targets
                    .iter()
                    .filter(|interaction| interaction.matches(&self.world, &context)),
            )
            .collect()
    }

    /// Query every verb currently applicable to `target`, for a
    /// point-and-click front-end's verb coin.
    ///
    /// The union of every [`Verb`] a currently-live *item-agnostic*
    /// interaction reports for `target` (`interactions_for(None,
    /// Some(target))` — this also picks up the NPC-hotspot `Verb::Talk` when
    /// `target` names a present NPC) with [`Rules::default_verbs`],
    /// deduplicated. An interaction gated on a specific carried item never
    /// contributes here, regardless of what the player holds: the coin is a
    /// pure function of the target and world state, never inventory-reactive
    /// (see AGENTS.md's north star item 4). Discovering that a specific
    /// carried item does something to `target` is a separate query —
    /// `interactions_for(Some(item), Some(target))` — fired when that item is
    /// actually used against it.
    #[must_use]
    pub fn verbs_for(&self, target: Target) -> HashSet<Verb> {
        let mut verbs: HashSet<Verb> = self
            .interactions_for(None, Some(target.clone()))
            .iter()
            .map(|interaction| interaction.verb())
            .collect();
        verbs.extend(self.rules.default_verbs(target, self.world()));
        verbs
    }

    /// Serialize the current game's *progress* (not the authored content)
    /// to a human-readable string.
    ///
    /// See [`GameEngine::load`] for what "progress" means and why it's a
    /// narrower thing than the whole `WorldState`.
    ///
    /// # Errors
    ///
    /// Returns a [`SaveError`] if the progress cannot be serialized.
    pub fn save(&self) -> Result<String, SaveError> {
        self.world.save()
    }

    /// Rebuild a `GameEngine` from `data` and `rules` — exactly as
    /// [`GameEngine::get_with_rules`] would, with fresh compiled
    /// interactions, fresh trigger definitions, and fresh NPC dialogue trees
    /// — then restore the progress captured by [`GameEngine::save`] on top
    /// of it: flags, player inventory, each room's object membership, door
    /// lock state, fired triggers, and NPC dialogue progress.
    ///
    /// `data` may be a different (patched) version of the world than `save`
    /// was taken against: static content always comes from `data`, never
    /// from the save, so a content change since the save was made is picked
    /// up. An object, room, or NPC id the save mentions that no longer
    /// exists in `data` is silently skipped rather than erroring.
    ///
    /// # Errors
    ///
    /// Returns a [`SaveError`] if `save` cannot be parsed.
    pub fn load(
        data: &WorldData,
        rules: impl Rules + 'static,
        save: &str,
    ) -> Result<GameEngine, SaveError> {
        let mut engine = Self::get_with_rules(data, rules);
        engine.world.restore(save)?;
        Ok(engine)
    }
}
