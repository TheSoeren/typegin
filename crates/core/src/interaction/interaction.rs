use crate::world::WorldState;
use crate::world::npc::NpcId;
use crate::world::object::ObjectId;

use super::{ActionContext, Target, TargetFilter, Verb};

/// The condition of an interaction: a pure predicate over the world and the
/// interaction context. Runs both when dispatching and (via the query API)
/// when a front-end asks what is currently possible.
pub type InteractionCondition = dyn Fn(&WorldState, &ActionContext) -> bool;

/// The effect of an interaction: the behaviour that runs when the interaction
/// fires, mutating the world through its `&mut WorldState` and returning the
/// events to report. The world is the only mutable state in the engine.
pub type InteractionEffect = dyn Fn(&mut WorldState, &ActionContext) -> Vec<crate::event::Event>;

/// A single authored interaction: when the player does *verb* with *object*
/// (optionally on a *target* matching the filter) and `condition` holds, run
/// `effect`.
///
/// This is the authoring surface for custom puzzle logic: behaviour is
/// declared once, and a front-end can also *query* which interactions are
/// currently live (see
/// [`GameEngine::interactions_for`](crate::GameEngine::interactions_for)) to
/// build verb menus or drop-targets for a point-and-click UI. Both closures
/// must route any state mutation through the `&mut WorldState` they are
/// given — the world is the only mutable state in the engine.
pub struct Interaction {
    verb: Verb,
    item: Option<ObjectId>,
    target: TargetFilter,
    condition: Option<Box<InteractionCondition>>,
    effect: Box<InteractionEffect>,
    /// When `Some`, this interaction denotes "talk to this NPC" — an NPC
    /// hotspot in the `interactions_for` query rather than an object verb.
    /// Dispatch of `Talk` still routes through `Rules::on_talk`; the entry
    /// exists so a point-and-click front-end sees the NPC as a live target.
    npc: Option<NpcId>,
}

impl Interaction {
    /// Build an interaction from its parts.
    ///
    /// * `verb` — which action triggers it.
    /// * `item` — the object the player must be using/carrying, or `None` to
    ///   match any.
    /// * `target` — coarse target kind filter.
    /// * `condition` — optional gate; runs before `effect` and (importantly)
    ///   also when a front-end *queries* available interactions, so the query
    ///   only reports things that currently make sense.
    /// * `effect` — the behaviour; returns the events to emit.
    #[must_use]
    pub fn build(
        verb: Verb,
        item: Option<ObjectId>,
        target: TargetFilter,
        condition: Option<Box<InteractionCondition>>,
        effect: Box<InteractionEffect>,
    ) -> Self {
        Interaction {
            verb,
            item,
            target,
            condition,
            effect,
            npc: None,
        }
    }

    /// Build a "talk to this NPC" interaction: a [`Verb::Talk`] hotspot for an
    /// NPC, live only while the player is in the NPC's room.
    ///
    /// No object coupling (`item` is always `None`, matching any queried
    /// item), but the condition does check the *target*: it matches a query
    /// naming this NPC specifically (`target: Some(Target::Npc(this_npc))`,
    /// any item), and also the fully open "what is clickable right now" query
    /// (`item: None, target: None`) a point-and-click UI uses to enumerate
    /// hotspots. It never matches a query targeted at something else. The
    /// effect is inert — a `Talk` action dispatches through `Rules::on_talk`,
    /// not through `Interaction` effects.
    #[must_use]
    pub fn talk_npc(npc: NpcId) -> Self {
        let present_npc = npc.clone();
        Interaction {
            verb: Verb::Talk,
            item: None,
            target: TargetFilter::Any,
            condition: Some(Box::new(
                move |world: &WorldState, context: &ActionContext| {
                    let target_ok = match &context.target {
                        Some(Target::Npc(id)) => id == &present_npc,
                        Some(Target::Object(_)) => false,
                        None => context.item.is_none(),
                    };
                    target_ok
                        && world
                            .npcs_in_room(&world.current_room_id())
                            .iter()
                            .any(|present| present.id() == &present_npc)
                },
            )),
            effect: Box::new(|_world: &mut WorldState, _context: &ActionContext| Vec::new()),
            npc: Some(npc),
        }
    }

    /// The verb this interaction reacts to.
    #[must_use]
    pub fn verb(&self) -> Verb {
        self.verb
    }

    /// The object this interaction requires (or `None` for "any").
    #[must_use]
    pub fn item(&self) -> Option<ObjectId> {
        self.item.clone()
    }

    /// The coarse target filter this interaction accepts.
    #[must_use]
    pub fn target(&self) -> TargetFilter {
        self.target
    }

    /// The NPC this interaction denotes talking to, when it is a [`Verb::Talk`]
    /// hotspot (as built by [`Interaction::talk_npc`]).
    #[must_use]
    pub fn npc(&self) -> Option<&NpcId> {
        self.npc.as_ref()
    }

    /// Whether this interaction applies to the given context under the given
    /// world state. Used both by the dispatcher (run it) and by the query
    /// API (list it).
    #[must_use]
    pub fn matches(&self, world: &WorldState, context: &ActionContext) -> bool {
        let item_ok = match &self.item {
            Some(id) => context.item.as_ref() == Some(id),
            None => true,
        };
        item_ok
            && context.verb.is_none_or(|v| self.verb() == v)
            && self.target.matches(world, context.target.as_ref())
            && self.condition_applies(world, context)
    }

    /// Run the interaction's effect and return the events it produced.
    pub fn run(&self, world: &mut WorldState, context: &ActionContext) -> Vec<crate::event::Event> {
        (self.effect)(world, context)
    }

    fn condition_applies(&self, world: &WorldState, context: &ActionContext) -> bool {
        match &self.condition {
            Some(condition) => condition(world, context),
            None => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Target;
    use crate::data::WorldData;
    use crate::data::npc_data::{DialogueData, NpcData};
    use crate::data::object_data::{ObjectData, ObjectKind};
    use crate::data::room_data::RoomData;
    use crate::keys::room_id::RoomId;
    use std::collections::HashMap;

    /// A one-room world containing a single `Item` object `sword`, and
    /// optionally an NPC `guard` living in that room.
    fn world_with(npc_in_room: bool) -> WorldState {
        let npcs = if npc_in_room {
            vec![NpcData {
                id: NpcId::new("guard"),
                primary_name: "guard".to_string(),
                aliases: Vec::new(),
                room: RoomId::new("room"),
                dialogue: DialogueData {
                    root: crate::keys::dialogue_node_id::DialogueNodeId::new("start"),
                    nodes: HashMap::new(),
                },
            }]
        } else {
            Vec::new()
        };
        let data = WorldData {
            flags: Vec::new(),
            objects: vec![ObjectData {
                id: ObjectId::new("sword"),
                primary_name: "sword".to_string(),
                aliases: Vec::new(),
                kind: ObjectKind::Item,
                door: None,
                extra: HashMap::new(),
            }],
            rooms: vec![RoomData {
                id: RoomId::new("room"),
                visible_objects: vec![ObjectId::new("sword")],
                hidden_objects: Vec::new(),
                extra: HashMap::new(),
            }],
            interactions: Vec::new(),
            npcs,
            triggers: Vec::new(),
        };
        WorldState::from_data(&data)
    }

    fn no_op_effect() -> Box<InteractionEffect> {
        Box::new(|_world: &mut WorldState, _context: &ActionContext| Vec::new())
    }

    #[test]
    fn matches_requires_the_verb_when_context_specifies_one() {
        let interaction =
            Interaction::build(Verb::Take, None, TargetFilter::Any, None, no_op_effect());
        let world = world_with(false);
        let take_context = ActionContext::new(Some(Verb::Take), None, None);
        let drop_context = ActionContext::new(Some(Verb::Drop), None, None);
        assert!(interaction.matches(&world, &take_context));
        assert!(!interaction.matches(&world, &drop_context));
    }

    #[test]
    fn matches_ignores_verb_when_context_has_none() {
        let interaction =
            Interaction::build(Verb::Take, None, TargetFilter::Any, None, no_op_effect());
        let world = world_with(false);
        let context = ActionContext::new(None, None, None);
        assert!(interaction.matches(&world, &context));
    }

    #[test]
    fn matches_requires_the_exact_item_when_one_is_set() {
        let interaction = Interaction::build(
            Verb::Take,
            Some(ObjectId::new("sword")),
            TargetFilter::Any,
            None,
            no_op_effect(),
        );
        let world = world_with(false);
        let matching = ActionContext::new(Some(Verb::Take), Some(ObjectId::new("sword")), None);
        let other_item = ActionContext::new(Some(Verb::Take), Some(ObjectId::new("shield")), None);
        let no_item = ActionContext::new(Some(Verb::Take), None, None);
        assert!(interaction.matches(&world, &matching));
        assert!(!interaction.matches(&world, &other_item));
        assert!(!interaction.matches(&world, &no_item));
    }

    #[test]
    fn matches_none_item_matches_any_carried_item() {
        let interaction =
            Interaction::build(Verb::Take, None, TargetFilter::Any, None, no_op_effect());
        let world = world_with(false);
        let with_item = ActionContext::new(Some(Verb::Take), Some(ObjectId::new("sword")), None);
        let no_item = ActionContext::new(Some(Verb::Take), None, None);
        assert!(interaction.matches(&world, &with_item));
        assert!(interaction.matches(&world, &no_item));
    }

    #[test]
    fn matches_defers_to_the_target_filter() {
        let interaction = Interaction::build(
            Verb::Use,
            None,
            TargetFilter::Targeted,
            None,
            no_op_effect(),
        );
        let world = world_with(false);
        let no_target = ActionContext::new(Some(Verb::Use), None, None);
        let with_target = ActionContext::new(
            Some(Verb::Use),
            None,
            Some(Target::Object(ObjectId::new("sword"))),
        );
        assert!(!interaction.matches(&world, &no_target));
        assert!(interaction.matches(&world, &with_target));
    }

    #[test]
    fn matches_runs_the_condition_when_present() {
        let interaction = Interaction::build(
            Verb::Take,
            None,
            TargetFilter::Any,
            Some(Box::new(|_world: &WorldState, _context: &ActionContext| {
                false
            })),
            no_op_effect(),
        );
        let world = world_with(false);
        let context = ActionContext::new(Some(Verb::Take), None, None);
        assert!(!interaction.matches(&world, &context));
    }

    #[test]
    fn condition_applies_defaults_to_true_when_absent() {
        let interaction =
            Interaction::build(Verb::Take, None, TargetFilter::Any, None, no_op_effect());
        let world = world_with(false);
        let context = ActionContext::new(Some(Verb::Take), None, None);
        assert!(interaction.condition_applies(&world, &context));
    }

    #[test]
    fn run_invokes_the_effect_and_returns_its_events() {
        let interaction = Interaction::build(
            Verb::Take,
            None,
            TargetFilter::Any,
            None,
            Box::new(|_world: &mut WorldState, _context: &ActionContext| {
                vec![crate::event::Event::Custom {
                    name: "ran".to_string(),
                }]
            }),
        );
        let mut world = world_with(false);
        let context = ActionContext::new(Some(Verb::Take), None, None);
        assert_eq!(
            interaction.run(&mut world, &context),
            vec![crate::event::Event::Custom {
                name: "ran".to_string()
            }]
        );
    }

    #[test]
    fn accessors_expose_the_built_parts() {
        let interaction = Interaction::build(
            Verb::Use,
            Some(ObjectId::new("sword")),
            TargetFilter::Scene,
            None,
            no_op_effect(),
        );
        assert_eq!(interaction.verb(), Verb::Use);
        assert_eq!(interaction.item(), Some(ObjectId::new("sword")));
        assert_eq!(interaction.target(), TargetFilter::Scene);
        assert_eq!(interaction.npc(), None);
    }

    #[test]
    fn talk_npc_matches_only_while_the_npc_is_in_the_current_room() {
        let interaction = Interaction::talk_npc(NpcId::new("guard"));
        assert_eq!(interaction.verb(), Verb::Talk);
        assert_eq!(interaction.npc(), Some(&NpcId::new("guard")));

        let present = world_with(true);
        let context = ActionContext::new(Some(Verb::Talk), None, None);
        assert!(interaction.matches(&present, &context));

        let absent = world_with(false);
        assert!(!interaction.matches(&absent, &context));
    }

    #[test]
    fn talk_npc_effect_is_inert() {
        let interaction = Interaction::talk_npc(NpcId::new("guard"));
        let mut world = world_with(true);
        let context = ActionContext::new(Some(Verb::Talk), None, None);
        assert_eq!(interaction.run(&mut world, &context), Vec::new());
    }
}
