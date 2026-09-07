use serde::Deserialize;

use crate::data::{WorldData, WorldDataError};
use crate::event::Event;
use crate::input::action::{DiscardResult, DropResult, GrantResult, TakeResult};
use crate::input::direction::{Direction, DirectionResolution};
use crate::interaction::{ActionContext, Interaction, TargetFilter, Verb};
use crate::world::WorldState;
use crate::world::object::ObjectId;
use crate::world::room::RoomId;

impl InteractionData {
    /// Verify that every key this interaction references exists in `data`.
    ///
    /// # Errors
    ///
    /// Returns a [`WorldDataError::Validation`] naming the first unknown key.
    pub(crate) fn validate_references(&self, data: &WorldData) -> Result<(), WorldDataError> {
        if let Some(item) = &self.item {
            data.find_object(item).ok_or_else(|| {
                WorldDataError::Validation(format!(
                    "interaction with verb `{:?}` references unknown object key `{}`",
                    self.verb, item
                ))
            })?;
        }
        if let Some(DataTarget::Object { object }) = &self.target {
            data.find_object(object).ok_or_else(|| {
                WorldDataError::Validation(format!(
                    "interaction with verb `{:?}` references unknown object key `{}`",
                    self.verb, object
                ))
            })?;
        }
        for condition in &self.condition {
            condition.validate_references(data)?;
        }
        for effect in &self.effect {
            effect.validate_references(data)?;
        }
        Ok(())
    }
}

/// A single authored interaction shipped in world data (YAML) instead of a
/// Rust closure: "when the player does *verb* with *object* (optionally on a
/// *target*) and every `condition` holds, run every `effect` in order".
///
/// This is the data-driven authoring surface (roadmap item #1). Matching and
/// dispatch mirror the closure [`Interaction`] surface — data interactions run
/// *before* `Rules::interactions()` closures and before the stock fallback.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct InteractionData {
    pub verb: Verb,
    #[serde(default)]
    pub item: Option<ObjectId>,
    #[serde(default)]
    pub target: Option<DataTarget>,
    #[serde(default)]
    pub condition: Vec<DataCondition>,
    #[serde(default)]
    pub effect: Vec<DataEffect>,
}

impl InteractionData {
    /// Whether this interaction applies to the given context.
    #[must_use]
    pub(crate) fn matches(&self, world: &WorldState, context: &ActionContext) -> bool {
        let item_ok = match &self.item {
            Some(id) => context.item.as_ref() == Some(id),
            None => true,
        };
        item_ok
            && context.verb.is_none_or(|verb| self.verb == verb)
            && self.target_matches(world, context.target.as_ref())
            && self.condition_applies(world, context)
    }

    fn target_matches(&self, world: &WorldState, target: Option<&ObjectId>) -> bool {
        match &self.target {
            None => true,
            Some(DataTarget::Object { object }) => target == Some(object),
            Some(DataTarget::Kind { kind }) => match kind {
                DataTargetKind::Scene => target.is_some_and(|id| world.object_is_scene(id)),
            },
        }
    }

    fn condition_applies(&self, world: &WorldState, context: &ActionContext) -> bool {
        self.condition
            .iter()
            .all(|condition| condition.matches(world, context))
    }

    /// Apply the interaction's effects in order, mutating the world through
    /// it and returning the events to report.
    #[must_use]
    pub(crate) fn run(&self, world: &mut WorldState) -> Vec<Event> {
        self.effect
            .iter()
            .filter_map(|effect| effect.apply(world))
            .collect()
    }

    /// Compile this data interaction into the closure [`Interaction`] shape
    /// used by the public query API (`GameEngine::interactions_for`). The
    /// closure condition re-evaluates the data conditions against the live
    /// world so the query only reports currently-possible interactions.
    #[must_use]
    pub(crate) fn compile(&self) -> Interaction {
        let (filter, exact_target) = match &self.target {
            None => (TargetFilter::Any, None),
            Some(DataTarget::Object { object }) => {
                let object = object.clone();
                (
                    TargetFilter::Targeted,
                    Some(
                        Box::new(move |_world: &WorldState, context: &ActionContext| {
                            context.target == Some(object.clone())
                        }) as Box<InteractionConditionFn>,
                    ),
                )
            }
            Some(DataTarget::Kind {
                kind: DataTargetKind::Scene,
            }) => (TargetFilter::Scene, None),
        };
        let condition_data = self.clone();
        let effect_data = self.clone();
        Interaction::build(
            self.verb,
            self.item.clone(),
            filter,
            Some(Box::new(
                move |world: &WorldState, context: &ActionContext| {
                    condition_data.condition_applies(world, context)
                        && exact_target.as_ref().is_none_or(|f| f(world, context))
                },
            )),
            Box::new(move |world: &mut WorldState, _context: &ActionContext| {
                effect_data.run(world)
            }),
        )
    }
}

/// A target spec for a data interaction. `target` is optional — omitted it
/// matches any target, including a self-use (`use X` with no target).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum DataTarget {
    /// Only the exact object with this id.
    Object { object: ObjectId },
    /// Any target of the coarse structural kind.
    Kind { kind: DataTargetKind },
}

/// Coarse *structural* target kind (matching [`TargetFilter`]). Properties of
/// a target — door-ness, lock state, ... — are expressed as conditions, not
/// kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DataTargetKind {
    /// A scene object (stays in the world; every door is one).
    Scene,
}

/// A pure predicate over the world and interaction context. Conditions AND
/// together; they gate dispatch *and* the `interactions_for` query.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum DataCondition {
    /// The player's current room has this key.
    Room { room: RoomId },
    /// The player is carrying this object.
    PlayerHolds { player_holds: ObjectId },
    /// The exit in `direction` from the current room is locked.
    ExitLocked { exit_locked: Direction },
    /// The exit in `direction` from the current room is hidden.
    ExitHidden { exit_hidden: Direction },
    /// The (present) target's door-ness equals the value.
    IsDoor { is_door: bool },
    /// Negation of any condition.
    Not { not: Box<DataCondition> },
}

impl DataCondition {
    #[must_use]
    fn matches(&self, world: &WorldState, context: &ActionContext) -> bool {
        match self {
            DataCondition::Room { room } => *room == world.current_room_id(),
            DataCondition::PlayerHolds { player_holds } => world.player_holds(player_holds),
            DataCondition::ExitLocked { exit_locked } => world.is_exit_locked(*exit_locked),
            DataCondition::ExitHidden { exit_hidden } => world.is_exit_hidden(*exit_hidden),
            DataCondition::IsDoor { is_door } => {
                context.target.as_ref().map(|id| world.object_is_door(id)) == Some(*is_door)
            }
            DataCondition::Not { not } => !not.matches(world, context),
        }
    }

    /// Verify that every key this condition references exists in `data`.
    ///
    /// # Errors
    ///
    /// Returns a [`WorldDataError::Validation`] naming the first unknown key.
    fn validate_references(&self, data: &WorldData) -> Result<(), WorldDataError> {
        match self {
            DataCondition::Room { room } => data.find_room(room).map(|_| ()).ok_or_else(|| {
                WorldDataError::Validation(format!(
                    "condition references unknown room key `{room}`",
                ))
            }),
            DataCondition::PlayerHolds { player_holds } => {
                data.find_object(player_holds).map(|_| ()).ok_or_else(|| {
                    WorldDataError::Validation(format!(
                        "condition references unknown object key `{player_holds}`",
                    ))
                })
            }
            DataCondition::Not { not } => not.validate_references(data),
            DataCondition::ExitLocked { .. }
            | DataCondition::ExitHidden { .. }
            | DataCondition::IsDoor { .. } => Ok(()),
        }
    }
}

/// An effect a data interaction runs when it fires. Effects run in order and
/// own the world mutation; silent effects emit no event.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum DataEffect {
    /// Emit a game-authored beat: `Event::Custom { name }`.
    Emit { emit: String },
    /// Move an object from the current room into inventory: `Event::Took`
    /// (no-op if the object is not in the room).
    Take { take: ObjectId },
    /// Add an object into inventory: `Event::Granted` (no-op if the object is already in inventory).
    Grant { grant: ObjectId },
    /// Move a carried object into the current room: `Event::Dropped` (no-op
    /// if the object is not carried).
    Drop { drop: ObjectId },
    /// Remove an object from inventory: `Event::Discarded` (no-op if the object is not carried).
    Discard { discard: ObjectId },
    /// Unlock the exit in `direction` from the current room:
    /// `Event::UnlockedExit` (no-op if no such exit).
    UnlockExit { unlock_exit: Direction },
    /// Lock the exit in `direction` from the current room (silent).
    LockExit { lock_exit: Direction },
    /// Reveal the hidden-door object standing in `direction` from the current
    /// room (silent).
    RevealExit { reveal_exit: Direction },
    /// Hide the door object standing in `direction` from the current room;
    /// the object becomes a hidden exit (silent).
    HideExit { hide_exit: Direction },
    /// Move an object from the current room's hidden set to visible (silent).
    RevealObject { reveal_object: ObjectId },
    /// Move an object from the current room's visible set to hidden (silent).
    HideObject { hide_object: ObjectId },
}

impl DataEffect {
    #[must_use]
    fn apply(&self, world: &mut WorldState) -> Option<Event> {
        match self {
            DataEffect::Emit { emit } => Some(Event::Custom { name: emit.clone() }),
            DataEffect::Take { take } => take_into_inventory(world, take.clone()),
            DataEffect::Grant { grant } => add_to_inventory(world, grant.clone()),
            DataEffect::Drop { drop } => drop_into_room(world, drop.clone()),
            DataEffect::Discard { discard } => remove_from_inventory(world, discard.clone()),
            DataEffect::UnlockExit { unlock_exit } => match world.unlock_exit(*unlock_exit) {
                DirectionResolution::Found(_) => Some(Event::UnlockedExit {
                    direction: *unlock_exit,
                }),
                DirectionResolution::NotFound => None,
            },
            DataEffect::LockExit { lock_exit } => {
                world.lock_exit(*lock_exit);
                None
            }
            DataEffect::RevealExit { reveal_exit } => {
                world.reveal_exit(*reveal_exit);
                None
            }
            DataEffect::HideExit { hide_exit } => {
                world.hide_exit(*hide_exit);
                None
            }
            DataEffect::RevealObject { reveal_object } => {
                world.reveal_object(reveal_object);
                None
            }
            DataEffect::HideObject { hide_object } => {
                world.hide_object(hide_object);
                None
            }
        }
    }

    /// Verify that every object key this effect references exists in `data`.
    ///
    /// # Errors
    ///
    /// Returns a [`WorldDataError::Validation`] naming the first unknown key.
    fn validate_references(&self, data: &WorldData) -> Result<(), WorldDataError> {
        let id = match self {
            DataEffect::Take { take } => take,
            DataEffect::Grant { grant } => grant,
            DataEffect::Drop { drop } => drop,
            DataEffect::Discard { discard } => discard,
            DataEffect::RevealObject { reveal_object } => reveal_object,
            DataEffect::HideObject { hide_object } => hide_object,
            DataEffect::Emit { .. }
            | DataEffect::UnlockExit { .. }
            | DataEffect::LockExit { .. }
            | DataEffect::RevealExit { .. }
            | DataEffect::HideExit { .. } => return Ok(()),
        };
        data.find_object(id).map(|_| ()).ok_or_else(|| {
            WorldDataError::Validation(format!("effect references unknown object key `{id}`"))
        })
    }
}

fn take_into_inventory(world: &mut WorldState, id: ObjectId) -> Option<Event> {
    let name = world.object_info(&id)?.name;
    match world.player_take_object(&id) {
        TakeResult::Success => Some(Event::Took {
            object_id: id,
            object: name,
        }),
        TakeResult::Fail => None,
    }
}

fn add_to_inventory(world: &mut WorldState, id: ObjectId) -> Option<Event> {
    match world.player_grant_object(&id) {
        GrantResult::Success => {
            // The object is in the inventory now, so it is in scope for the name.
            let name = world.object_info(&id)?.name;
            Some(Event::Granted {
                object_id: id,
                object: name,
            })
        }
        GrantResult::Fail => None,
    }
}

fn drop_into_room(world: &mut WorldState, id: ObjectId) -> Option<Event> {
    let name = world.object_info(&id)?.name;
    match world.player_drop_object(&id) {
        DropResult::Success => Some(Event::Dropped {
            object_id: id,
            object: name,
        }),
        DropResult::Fail => None,
    }
}

fn remove_from_inventory(world: &mut WorldState, id: ObjectId) -> Option<Event> {
    let name = world.object_info(&id)?.name;
    match world.player_discard_object(&id) {
        DiscardResult::Success => Some(Event::Discarded {
            object_id: id,
            object: name,
        }),
        DiscardResult::Fail => None,
    }
}

/// The top-level shape of an `interactions.yaml` file.
#[derive(Debug, Deserialize)]
pub(crate) struct InteractionsFile {
    #[serde(default)]
    pub(crate) interactions: Vec<InteractionData>,
}

type InteractionConditionFn = dyn Fn(&WorldState, &ActionContext) -> bool;

/// Run the first data interaction that matches `context`, if any. The matching
/// interaction fully owns the world mutation and the returned events.
#[must_use]
pub(crate) fn dispatch_data(world: &mut WorldState, context: &ActionContext) -> Option<Vec<Event>> {
    let position = world
        .data_interactions()
        .iter()
        .position(|interaction| interaction.matches(world, context))?;
    let interaction = world.data_interactions()[position].clone();
    Some(interaction.run(world))
}
