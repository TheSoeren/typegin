use crate::input::action::Action;
use crate::world::WorldState;
use crate::world::object::ObjectId;

/// The game's action vocabulary. An author writes interactions *for a verb*,
/// and a point-and-click front-end can enumerate the verbs an object accepts
/// instead of guessing from prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Verb {
    Look,
    Go,
    Examine,
    Take,
    Drop,
    Use,
    Any,
}

impl Verb {
    /// The verb a parsed [`Action`] maps to, when it maps to one at all.
    ///
    /// `Unknown` actions have no verb; `Use` with or without a target is the
    /// same verb (the target lives in the context, not the verb).
    pub fn from_action(action: &Action) -> Option<Verb> {
        match action {
            Action::Look => Some(Verb::Look),
            Action::Go(_) => Some(Verb::Go),
            Action::Examine(_) => Some(Verb::Examine),
            Action::Take(_) => Some(Verb::Take),
            Action::Drop(_) => Some(Verb::Drop),
            Action::Use { .. } => Some(Verb::Use),
            Action::Unknown(_) => None,
        }
    }
}

/// Runtime context handed to an interaction: which object was used/dropped and
/// which target (if any) the verb was directed at.
///
/// For `use X on Y`: `item` is the carried `X`, `target` is the resolved `Y`.
/// For a self-use (`use X`), `target` is `None`.
#[derive(Debug, Clone)]
pub struct ActionContext {
    pub verb: Option<Verb>,
    pub item: Option<ObjectId>,
    pub target: Option<ObjectId>,
}

impl ActionContext {
    pub fn new(verb: Option<Verb>, item: Option<ObjectId>, target: Option<ObjectId>) -> Self {
        ActionContext { verb, item, target }
    }
}

/// Coarse kind filter deciding which targets an interaction applies to. The
/// selection on top of it lives in the interaction's `condition`, which can
/// inspect the concrete target (a door's direction, an object's id, state,
/// ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetFilter {
    /// Any target, including no target at all (self-use).
    Any,
    /// Only a targeted interaction (use X on Y); no self-use.
    Targeted,
    /// Only use-with on a *scene* object (stays in the world).
    Scene,
    /// Only use-with on a *door* (a scene object carrying door data).
    Door,
}

impl TargetFilter {
    pub(crate) fn matches(&self, world: &WorldState, target: Option<ObjectId>) -> bool {
        match self {
            TargetFilter::Any => true,
            TargetFilter::Targeted => target.is_some(),
            TargetFilter::Scene => target.is_some_and(|id| world.object_is_scene(id)),
            TargetFilter::Door => target.is_some_and(|id| world.object_is_door(id)),
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
        }
    }

    /// The verb this interaction reacts to.
    pub fn verb(&self) -> Verb {
        self.verb
    }

    /// The object this interaction requires (or `None` for "any").
    pub fn item(&self) -> Option<ObjectId> {
        self.item
    }

    /// The coarse target filter this interaction accepts.
    pub fn target(&self) -> TargetFilter {
        self.target
    }

    /// Whether this interaction applies to the given context under the given
    /// world state. Used both by the dispatcher (run it) and by the query
    /// API (list it).
    pub fn matches(&self, world: &WorldState, context: &ActionContext) -> bool {
        let item_ok = match self.item {
            Some(id) => context.item == Some(id),
            None => true,
        };
        item_ok
            && context.verb.is_none_or(|v| self.verb() == v)
            && self.target.matches(world, context.target)
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
