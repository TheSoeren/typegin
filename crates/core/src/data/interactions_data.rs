use serde::Deserialize;

use crate::data::{WorldData, WorldDataError};
use crate::input::direction::Direction;
use crate::interaction::Verb;
use crate::keys::npc_id::NpcId;
use crate::keys::object_id::ObjectId;
use crate::keys::room_id::RoomId;

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
        if let Some(DataTarget::Npc { npc }) = &self.target
            && !data.npcs.iter().any(|npc_data| &npc_data.id == npc)
        {
            return Err(WorldDataError::Validation(format!(
                "interaction with verb `{:?}` references unknown npc key `{}`",
                self.verb, npc
            )));
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
/// Rust closure: when the player does *verb* with *object* (optionally on a
/// *target*) and every `condition` holds, run every `effect` in order.
///
/// Matching and dispatch mirror the closure
/// [`Interaction`](crate::interaction::Interaction) surface — data
/// interactions run *before* `Rules::interactions()` closures and before the
/// stock fallback; that runtime behaviour lives in `interaction::data_interactions`,
/// not here. This module is schema and structural validation only, like every
/// other `*_data` module.
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

/// A target spec for a data interaction. `target` is optional — omitted it
/// matches any target, including a self-use (`use X` with no target).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum DataTarget {
    /// Only the exact object with this id.
    Object { object: ObjectId },
    /// Only the exact NPC with this id (a "use item on guard" target).
    Npc { npc: NpcId },
    /// Any target of the coarse structural kind.
    Kind { kind: DataTargetKind },
}

/// Coarse *structural* target kind (matching
/// [`TargetFilter`](crate::interaction::TargetFilter)). Properties of a
/// target — door-ness, lock state, ... — are expressed as conditions, not
/// kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DataTargetKind {
    /// A scene object (stays in the world; every door is one).
    Scene,
}

/// A pure predicate over the world and interaction context. Conditions AND
/// together; they gate dispatch *and* the `interactions_for` query.
///
/// Evaluated against live [`WorldState`](crate::world::WorldState) by
/// `interaction::data_interactions`.
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
    /// The global flag is enabled.
    Flag { flag: String },
    /// Negation of any condition.
    Not { not: Box<DataCondition> },
}

impl DataCondition {
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
            | DataCondition::IsDoor { .. }
            | DataCondition::Flag { .. } => Ok(()),
        }
    }
}

/// An effect a data interaction runs when it fires. Effects run in order and
/// own the world mutation; silent effects emit no event.
///
/// Applied against live [`WorldState`](crate::world::WorldState) by
/// `interaction::data_interactions`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum DataEffect {
    /// Emit a game-authored beat: `Event::Custom { name }`.
    Emit { emit: String },
    /// Move an object from the current room into inventory: `Event::Took`
    /// (no-op if the object is not in the room).
    Take { take: ObjectId },
    /// Add an object into inventory: `Event::Granted` (no-op if the object is
    /// already in inventory).
    Grant { grant: ObjectId },
    /// Move a carried object into the current room: `Event::Dropped` (no-op
    /// if the object is not carried).
    Drop { drop: ObjectId },
    /// Remove an object from inventory: `Event::Discarded` (no-op if the
    /// object is not carried).
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
    /// Set a global flag to true.
    SetFlag {
        #[serde(rename = "set_flag")]
        flag: String,
    },
    /// Set a global flag to false.
    ClearFlag {
        #[serde(rename = "clear_flag")]
        flag: String,
    },
}

impl DataEffect {
    /// Verify that every object key this effect references exists in `data`.
    ///
    /// # Errors
    ///
    /// Returns a [`WorldDataError::Validation`] naming the first unknown key.
    pub(crate) fn validate_references(&self, data: &WorldData) -> Result<(), WorldDataError> {
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
            | DataEffect::HideExit { .. }
            | DataEffect::SetFlag { .. }
            | DataEffect::ClearFlag { .. } => return Ok(()),
        };
        data.find_object(id).map(|_| ()).ok_or_else(|| {
            WorldDataError::Validation(format!("effect references unknown object key `{id}`"))
        })
    }
}
