use serde::Deserialize;

use crate::input::action::Action;
use crate::world::WorldState;
use crate::world::npc::NpcId;
use crate::world::object::ObjectId;

/// The game's action vocabulary. An author writes interactions *for a verb*,
/// and a point-and-click front-end can enumerate the verbs an object accepts
/// instead of guessing from prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verb {
    Look,
    Go,
    Examine,
    Take,
    Drop,
    Use,
    Talk,
}

impl Verb {
    /// The verb a parsed [`Action`] maps to, when it maps to one at all.
    ///
    /// `Unknown` actions have no verb; `Use` with or without a target is the
    /// same verb (the target lives in the context, not the verb).
    #[must_use]
    pub fn from_action(action: &Action) -> Option<Verb> {
        match action {
            Action::Look => Some(Verb::Look),
            Action::Go(_) => Some(Verb::Go),
            Action::Examine(_) => Some(Verb::Examine),
            Action::Take(_) => Some(Verb::Take),
            Action::Drop(_) => Some(Verb::Drop),
            Action::Use { .. } => Some(Verb::Use),
            Action::Talk(_) => Some(Verb::Talk),
            Action::Choose(_) | Action::Unknown(_) => None,
        }
    }
}

/// A resolved `Use`-with target: either a world object or an NPC in the
/// current room.
///
/// NPCs are deliberately *not* objects — there is no coercion between the two
/// types — but a use-with target may be either. `Target` is what lets an item
/// be used on a character ("use mallet on guard") and what lets a
/// point-and-click front-end offer NPCs as drop-targets in
/// [`GameEngine::interactions_for`](crate::GameEngine::interactions_for).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// A world object.
    Object(ObjectId),
    /// An NPC in the current room.
    Npc(NpcId),
}

/// Runtime context handed to an interaction: which object was used/dropped and
/// which target (if any) the verb was directed at.
///
/// For `use X on Y`: `item` is the carried `X`, `target` is the resolved `Y`
/// (object or NPC). For a self-use (`use X`), `target` is `None`.
#[derive(Debug, Clone)]
pub struct ActionContext {
    pub verb: Option<Verb>,
    pub item: Option<ObjectId>,
    pub target: Option<Target>,
}

impl ActionContext {
    #[must_use]
    pub fn new(verb: Option<Verb>, item: Option<ObjectId>, target: Option<Target>) -> Self {
        ActionContext { verb, item, target }
    }

    /// The object this context's target denotes, when the target is an object.
    #[must_use]
    pub fn target_object(&self) -> Option<&ObjectId> {
        match &self.target {
            Some(Target::Object(id)) => Some(id),
            _ => None,
        }
    }

    /// The NPC this context's target denotes, when the target is an NPC.
    #[must_use]
    pub fn target_npc(&self) -> Option<&NpcId> {
        match &self.target {
            Some(Target::Npc(id)) => Some(id),
            _ => None,
        }
    }
}

/// Coarse structural filter deciding which targets an interaction applies to:
/// arity (`Any` vs `Targeted`) and world-position (`Scene`). The task-specific
/// selection on top of it lives in the interaction's `condition`, which can
/// inspect the concrete target (a door's direction, its door-ness, lock state,
/// ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetFilter {
    /// Any target, including no target at all (self-use).
    Any,
    /// Only a targeted interaction (use X on Y); no self-use.
    Targeted,
    /// Only use-with on a *scene* object (stays in the world).
    Scene,
}

impl TargetFilter {
    pub(crate) fn matches(self, world: &WorldState, target: Option<&Target>) -> bool {
        match self {
            TargetFilter::Any => true,
            TargetFilter::Targeted => target.is_some(),
            TargetFilter::Scene => target.is_some_and(
                |target| matches!(target, Target::Object(id) if world.object_is_scene(id)),
            ),
        }
    }
}

/// The condition of an interaction: a pure predicate over the world and the
/// interaction context. Runs both when dispatching and (via the query API)
/// when a front-end asks what is currently possible.
pub type InteractionCondition = dyn Fn(&WorldState, &ActionContext) -> bool;

/// The effect of an interaction: the behaviour that runs when the interaction
/// fires, mutating the world through its `&mut WorldState` and returning the
/// events to report. The world is the only mutable state in the engine.
pub type InteractionEffect = dyn Fn(&mut WorldState, &ActionContext) -> Vec<crate::event::Event>;

/// A single authored interaction: "when the player does *verb* with *object*
/// (optionally on a *target* matching the filter) and the `condition` holds,
/// run `effect`.
///
/// This is the authoring surface for custom puzzle logic (Visionaire-style):
/// per-interaction behaviour is declared once and the engine dispatches to it
/// — and a front-end can *query* which interactions are currently live (see
/// [`GameEngine::interactions_for`](crate::GameEngine::interactions_for)) to
/// build verb menus or drop-targets for a point-and-click UI.
///
/// Both closures are pure with respect to captured state: any state mutation
/// must go through the `&mut WorldState` (the world is the only mutable
/// state in the engine).
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
    /// No object coupling: the entry carries no `item` or `target` filter, so
    /// it only surfaces in the open `interactions_for(None, None)` query a
    /// point-and-click UI uses to enumerate what is clickable right now. The
    /// effect is inert — a `Talk` action dispatches through
    /// `Rules::on_talk`, not through `Interaction` effects.
    #[must_use]
    pub fn talk_npc(npc: NpcId) -> Self {
        let present_npc = npc.clone();
        Interaction {
            verb: Verb::Talk,
            item: None,
            target: TargetFilter::Any,
            condition: Some(Box::new(
                move |world: &WorldState, _context: &ActionContext| {
                    world
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
            None => context.item.is_none(),
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
