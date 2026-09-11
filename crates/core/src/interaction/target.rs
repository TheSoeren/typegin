use serde::Deserialize;

use crate::world::WorldState;
use crate::world::npc::NpcId;
use crate::world::object::ObjectId;

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

/// Outcome of resolving a player-typed noun against a use-with target
/// (object or NPC) in scope.
pub type TargetResolution = crate::keys::Resolution<Target>;

/// A structural *world-position* class a use-with target can belong to.
/// Properties of a target — door-ness, lock state, ... — are expressed as
/// conditions, not kinds.
///
/// Deserialized directly from authored YAML (`target: kind: scene`) as
/// [`DataTarget::Kind`](crate::data::interactions_data::DataTarget::Kind),
/// and reused as-is by [`TargetFilter::Kind`] at runtime — one type spans the
/// authored schema and the compiled shape, the same way
/// [`Verb`](crate::interaction::Verb) does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TargetKind {
    /// A scene object (stays in the world; every door is one).
    Scene,
    /// An object currently in the player's inventory.
    Carried,
}

/// Coarse structural filter deciding which targets an interaction applies to:
/// arity (`Any` vs `Targeted`) and world-position (`Kind`). The task-specific
/// selection on top of it lives in the interaction's `condition`, which can
/// inspect the concrete target (a door's direction, its door-ness, lock state,
/// ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetFilter {
    /// Any target, including no target at all (self-use).
    Any,
    /// Only a targeted interaction (use X on Y); no self-use.
    Targeted,
    /// Only use-with on a target of this structural world-position kind.
    Kind(TargetKind),
}

impl TargetFilter {
    pub(crate) fn matches(self, world: &WorldState, target: Option<&Target>) -> bool {
        match self {
            TargetFilter::Any => true,
            TargetFilter::Targeted => target.is_some(),
            TargetFilter::Kind(TargetKind::Scene) => target.is_some_and(
                |target| matches!(target, Target::Object(id) if world.object_is_scene(id)),
            ),
            TargetFilter::Kind(TargetKind::Carried) => target.is_some_and(
                |target| matches!(target, Target::Object(id) if world.player_holds(id)),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::WorldData;
    use crate::data::object_data::{ObjectData, ObjectKind};
    use crate::data::room_data::RoomData;
    use crate::keys::object_id::ObjectId;
    use crate::keys::room_id::RoomId;
    use crate::world::npc::NpcId;
    use std::collections::HashMap;

    /// A one-room world with one `Item` and one `Scene` object, both visible.
    fn world() -> WorldState {
        let data = WorldData {
            flags: Vec::new(),
            objects: vec![
                ObjectData {
                    id: ObjectId::new("sword"),
                    primary_name: "sword".to_string(),
                    aliases: Vec::new(),
                    kind: ObjectKind::Item,
                    door: None,
                    extra: HashMap::new(),
                },
                ObjectData {
                    id: ObjectId::new("cabinet"),
                    primary_name: "cabinet".to_string(),
                    aliases: Vec::new(),
                    kind: ObjectKind::Scene,
                    door: None,
                    extra: HashMap::new(),
                },
            ],
            rooms: vec![RoomData {
                id: RoomId::new("room"),
                visible_objects: vec![ObjectId::new("sword"), ObjectId::new("cabinet")],
                hidden_objects: Vec::new(),
                extra: HashMap::new(),
            }],
            interactions: Vec::new(),
            npcs: Vec::new(),
            triggers: Vec::new(),
        };
        WorldState::from_data(&data)
    }

    #[test]
    fn any_matches_regardless_of_target() {
        let world = world();
        assert!(TargetFilter::Any.matches(&world, None));
        assert!(TargetFilter::Any.matches(&world, Some(&Target::Object(ObjectId::new("sword")))));
    }

    #[test]
    fn targeted_requires_some_target() {
        let world = world();
        assert!(!TargetFilter::Targeted.matches(&world, None));
        assert!(
            TargetFilter::Targeted.matches(&world, Some(&Target::Object(ObjectId::new("sword"))))
        );
        assert!(TargetFilter::Targeted.matches(&world, Some(&Target::Npc(NpcId::new("guard")))));
    }

    #[test]
    fn scene_requires_a_scene_object_target() {
        let world = world();
        let scene = TargetFilter::Kind(TargetKind::Scene);
        assert!(!scene.matches(&world, None));
        assert!(scene.matches(&world, Some(&Target::Object(ObjectId::new("cabinet")))));
        assert!(!scene.matches(&world, Some(&Target::Object(ObjectId::new("sword")))));
    }

    #[test]
    fn scene_rejects_an_npc_target() {
        let world = world();
        assert!(
            !TargetFilter::Kind(TargetKind::Scene)
                .matches(&world, Some(&Target::Npc(NpcId::new("guard"))))
        );
    }
}
