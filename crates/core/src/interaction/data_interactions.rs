//! Runtime dispatch for authored [`InteractionData`]: matching a live
//! [`WorldState`] and [`ActionContext`], applying effects, and compiling into
//! the closure [`Interaction`] shape for the query API.
//!
//! `data::interactions_data` is schema only (like every other `*_data`
//! module); this is the behaviour that schema drives, so it lives on the
//! `interaction` side of the data/world boundary instead.

use crate::data::interactions_data::{
    DataCondition, DataEffect, DataTarget, DataTargetKind, InteractionData,
};
use crate::event::Event;
use crate::input::action::{DiscardResult, DropResult, GrantResult, TakeResult};
use crate::input::direction::DirectionResolution;
use crate::interaction::{ActionContext, Interaction, Target, TargetFilter};
use crate::model::object_id::ObjectId;
use crate::world::WorldState;

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

    fn target_matches(&self, world: &WorldState, target: Option<&Target>) -> bool {
        match &self.target {
            None => target.is_none(),
            Some(DataTarget::Object { object }) => target == Some(&Target::Object(object.clone())),
            Some(DataTarget::Npc { npc }) => target == Some(&Target::Npc(npc.clone())),
            Some(DataTarget::Kind { kind }) => match kind {
                DataTargetKind::Scene => target.is_some_and(
                    |target| matches!(target, Target::Object(id) if world.object_is_scene(id)),
                ),
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
        DataEffect::apply_all(&self.effect, world)
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
                            context.target == Some(Target::Object(object.clone()))
                        }) as Box<InteractionConditionFn>,
                    ),
                )
            }
            Some(DataTarget::Npc { npc }) => {
                let npc = npc.clone();
                (
                    TargetFilter::Targeted,
                    Some(
                        Box::new(move |_world: &WorldState, context: &ActionContext| {
                            context.target == Some(Target::Npc(npc.clone()))
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

impl DataCondition {
    #[must_use]
    fn matches(&self, world: &WorldState, context: &ActionContext) -> bool {
        match self {
            DataCondition::Room { room } => *room == world.current_room_id(),
            DataCondition::PlayerHolds { player_holds } => world.player_holds(player_holds),
            DataCondition::ExitLocked { exit_locked } => world.is_exit_locked(*exit_locked),
            DataCondition::ExitHidden { exit_hidden } => world.is_exit_hidden(*exit_hidden),
            DataCondition::IsDoor { is_door } => {
                context.target_object().map(|id| world.object_is_door(id)) == Some(*is_door)
            }
            DataCondition::Flag { flag } => world.has_flag(flag),
            DataCondition::Not { not } => !not.matches(world, context),
        }
    }
}

impl DataEffect {
    /// Apply a list of effects in order, mutating the world and returning the
    /// events they emit. Silent effects emit no event.
    #[must_use]
    pub(crate) fn apply_all(effects: &[DataEffect], world: &mut WorldState) -> Vec<Event> {
        effects
            .iter()
            .filter_map(|effect| effect.apply(world))
            .collect()
    }

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
            DataEffect::SetFlag { flag } => {
                world.set_flag(flag);
                Some(Event::FlagSet { flag: flag.clone() })
            }
            DataEffect::ClearFlag { flag } => {
                world.clear_flag(flag);
                Some(Event::FlagCleared { flag: flag.clone() })
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::door_data::DoorData;
    use crate::data::object_data::{ObjectData, ObjectKind};
    use crate::data::room_data::RoomData;
    use crate::input::direction::Direction;
    use crate::interaction::Verb;
    use crate::model::room_id::RoomId;
    use std::collections::HashMap;

    fn item(id: &str) -> ObjectData {
        ObjectData {
            id: ObjectId::new(id),
            primary_name: id.to_string(),
            aliases: Vec::new(),
            kind: ObjectKind::Item,
            door: None,
            extra: HashMap::new(),
        }
    }

    fn door(id: &str, direction: &str, to: &str, locked: bool) -> ObjectData {
        ObjectData {
            id: ObjectId::new(id),
            primary_name: id.to_string(),
            aliases: Vec::new(),
            kind: ObjectKind::Scene,
            door: Some(DoorData {
                direction: direction.to_string(),
                to: to.to_string(),
                locked,
            }),
            extra: HashMap::new(),
        }
    }

    fn scene(id: &str) -> ObjectData {
        ObjectData {
            id: ObjectId::new(id),
            primary_name: id.to_string(),
            aliases: Vec::new(),
            kind: ObjectKind::Scene,
            door: None,
            extra: HashMap::new(),
        }
    }

    /// A two-room fixture: `room` (start) has a visible `sword`, `lamp`,
    /// `torch` and `cabinet` (a non-door scene object), a visible unlocked
    /// `east-door` and a visible locked `north-door`, plus a hidden `chest`
    /// and a hidden unlocked `south-door` (both leading to `other`). `shield`
    /// is declared but placed in no room, so it is only reachable via
    /// `grant`.
    fn world_with_interactions(interactions: Vec<InteractionData>) -> WorldState {
        let data = crate::data::WorldData {
            flags: Vec::new(),
            objects: vec![
                item("sword"),
                item("lamp"),
                item("torch"),
                item("chest"),
                item("shield"),
                scene("cabinet"),
                door("north-door", "north", "other", true),
                door("east-door", "east", "other", false),
                door("south-door", "south", "other", false),
            ],
            rooms: vec![
                RoomData {
                    id: RoomId::new("room"),
                    visible_objects: vec![
                        ObjectId::new("sword"),
                        ObjectId::new("lamp"),
                        ObjectId::new("torch"),
                        ObjectId::new("cabinet"),
                        ObjectId::new("north-door"),
                        ObjectId::new("east-door"),
                    ],
                    hidden_objects: vec![ObjectId::new("chest"), ObjectId::new("south-door")],
                    extra: HashMap::new(),
                },
                RoomData {
                    id: RoomId::new("other"),
                    visible_objects: Vec::new(),
                    hidden_objects: Vec::new(),
                    extra: HashMap::new(),
                },
            ],
            interactions,
            npcs: Vec::new(),
        };
        WorldState::from_data(&data)
    }

    fn world() -> WorldState {
        world_with_interactions(Vec::new())
    }

    fn no_target_context(verb: Verb) -> ActionContext {
        ActionContext::new(Some(verb), None, None)
    }

    // -- DataCondition::matches --

    #[test]
    fn room_condition_matches_only_the_current_room() {
        let world = world();
        let context = no_target_context(Verb::Examine);
        assert!(
            DataCondition::Room {
                room: RoomId::new("room")
            }
            .matches(&world, &context)
        );
        assert!(
            !DataCondition::Room {
                room: RoomId::new("other")
            }
            .matches(&world, &context)
        );
    }

    #[test]
    fn player_holds_condition_tracks_inventory() {
        let mut world = world();
        let context = no_target_context(Verb::Examine);
        let condition = DataCondition::PlayerHolds {
            player_holds: ObjectId::new("sword"),
        };
        assert!(!condition.matches(&world, &context));
        world.player_take_object(&ObjectId::new("sword"));
        assert!(condition.matches(&world, &context));
    }

    #[test]
    fn exit_locked_condition_reflects_door_lock_state() {
        let world = world();
        let context = no_target_context(Verb::Examine);
        assert!(
            DataCondition::ExitLocked {
                exit_locked: Direction::North
            }
            .matches(&world, &context)
        );
        assert!(
            !DataCondition::ExitLocked {
                exit_locked: Direction::East
            }
            .matches(&world, &context)
        );
    }

    #[test]
    fn exit_hidden_condition_reflects_hidden_door_membership() {
        let world = world();
        let context = no_target_context(Verb::Examine);
        assert!(
            DataCondition::ExitHidden {
                exit_hidden: Direction::South
            }
            .matches(&world, &context)
        );
        assert!(
            !DataCondition::ExitHidden {
                exit_hidden: Direction::North
            }
            .matches(&world, &context)
        );
    }

    #[test]
    fn is_door_condition_checks_the_target() {
        let world = world();
        let door_context = ActionContext::new(
            Some(Verb::Examine),
            None,
            Some(Target::Object(ObjectId::new("north-door"))),
        );
        let non_door_context = ActionContext::new(
            Some(Verb::Examine),
            None,
            Some(Target::Object(ObjectId::new("sword"))),
        );
        assert!(DataCondition::IsDoor { is_door: true }.matches(&world, &door_context));
        assert!(!DataCondition::IsDoor { is_door: false }.matches(&world, &door_context));
        assert!(DataCondition::IsDoor { is_door: false }.matches(&world, &non_door_context));
    }

    #[test]
    fn is_door_condition_never_matches_without_a_target() {
        let world = world();
        let context = no_target_context(Verb::Examine);
        assert!(!DataCondition::IsDoor { is_door: true }.matches(&world, &context));
        assert!(!DataCondition::IsDoor { is_door: false }.matches(&world, &context));
    }

    #[test]
    fn flag_condition_tracks_world_flags() {
        let mut world = world();
        let context = no_target_context(Verb::Examine);
        let condition = DataCondition::Flag {
            flag: "quest-started".to_string(),
        };
        assert!(!condition.matches(&world, &context));
        world.set_flag("quest-started");
        assert!(condition.matches(&world, &context));
        world.clear_flag("quest-started");
        assert!(!condition.matches(&world, &context));
    }

    #[test]
    fn not_condition_inverts_the_inner_condition() {
        let world = world();
        let context = no_target_context(Verb::Examine);
        let condition = DataCondition::Not {
            not: Box::new(DataCondition::Room {
                room: RoomId::new("other"),
            }),
        };
        assert!(condition.matches(&world, &context));
    }

    // -- DataEffect::apply / apply_all --

    #[test]
    fn emit_effect_produces_a_custom_event() {
        let mut world = world();
        let events = DataEffect::apply_all(
            &[DataEffect::Emit {
                emit: "beat".to_string(),
            }],
            &mut world,
        );
        assert_eq!(
            events,
            vec![Event::Custom {
                name: "beat".to_string()
            }]
        );
    }

    #[test]
    fn take_effect_moves_a_visible_room_object_into_inventory() {
        let mut world = world();
        let events = DataEffect::apply_all(
            &[DataEffect::Take {
                take: ObjectId::new("sword"),
            }],
            &mut world,
        );
        assert_eq!(
            events,
            vec![Event::Took {
                object_id: ObjectId::new("sword"),
                object: "sword".to_string()
            }]
        );
        assert!(world.player_holds(&ObjectId::new("sword")));
    }

    #[test]
    fn take_effect_is_silent_for_a_hidden_object() {
        let mut world = world();
        let events = DataEffect::apply_all(
            &[DataEffect::Take {
                take: ObjectId::new("chest"),
            }],
            &mut world,
        );
        assert!(events.is_empty());
        assert!(!world.player_holds(&ObjectId::new("chest")));
    }

    #[test]
    fn grant_effect_materialises_an_item_placed_nowhere() {
        let mut world = world();
        let events = DataEffect::apply_all(
            &[DataEffect::Grant {
                grant: ObjectId::new("shield"),
            }],
            &mut world,
        );
        assert_eq!(
            events,
            vec![Event::Granted {
                object_id: ObjectId::new("shield"),
                object: "shield".to_string()
            }]
        );
        assert!(world.player_holds(&ObjectId::new("shield")));
    }

    #[test]
    fn grant_effect_is_a_no_op_when_already_held() {
        let mut world = world();
        world.player_grant_object(&ObjectId::new("shield"));
        let events = DataEffect::apply_all(
            &[DataEffect::Grant {
                grant: ObjectId::new("shield"),
            }],
            &mut world,
        );
        assert!(events.is_empty());
    }

    #[test]
    fn drop_effect_moves_a_carried_object_into_the_room() {
        let mut world = world();
        world.player_take_object(&ObjectId::new("sword"));
        let events = DataEffect::apply_all(
            &[DataEffect::Drop {
                drop: ObjectId::new("sword"),
            }],
            &mut world,
        );
        assert_eq!(
            events,
            vec![Event::Dropped {
                object_id: ObjectId::new("sword"),
                object: "sword".to_string()
            }]
        );
        assert!(!world.player_holds(&ObjectId::new("sword")));
    }

    #[test]
    fn drop_effect_is_a_no_op_when_not_carried() {
        let mut world = world();
        let events = DataEffect::apply_all(
            &[DataEffect::Drop {
                drop: ObjectId::new("sword"),
            }],
            &mut world,
        );
        assert!(events.is_empty());
    }

    #[test]
    fn discard_effect_removes_a_carried_object_without_placing_it() {
        let mut world = world();
        world.player_take_object(&ObjectId::new("sword"));
        let events = DataEffect::apply_all(
            &[DataEffect::Discard {
                discard: ObjectId::new("sword"),
            }],
            &mut world,
        );
        assert_eq!(
            events,
            vec![Event::Discarded {
                object_id: ObjectId::new("sword"),
                object: "sword".to_string()
            }]
        );
        assert!(!world.player_holds(&ObjectId::new("sword")));
        assert!(!world.room_object_names().contains(&"sword".to_string()));
    }

    #[test]
    fn discard_effect_is_a_no_op_when_not_carried() {
        let mut world = world();
        let events = DataEffect::apply_all(
            &[DataEffect::Discard {
                discard: ObjectId::new("sword"),
            }],
            &mut world,
        );
        assert!(events.is_empty());
    }

    #[test]
    fn unlock_exit_effect_unlocks_and_is_a_no_op_once_unlocked() {
        let mut world = world();
        let events = DataEffect::apply_all(
            &[DataEffect::UnlockExit {
                unlock_exit: Direction::North,
            }],
            &mut world,
        );
        assert_eq!(
            events,
            vec![Event::UnlockedExit {
                direction: Direction::North
            }]
        );
        assert!(!world.is_exit_locked(Direction::North));

        let events = DataEffect::apply_all(
            &[DataEffect::UnlockExit {
                unlock_exit: Direction::North,
            }],
            &mut world,
        );
        assert!(events.is_empty());
    }

    #[test]
    fn lock_exit_effect_is_silent_and_locks_the_door() {
        let mut world = world();
        assert!(!world.is_exit_locked(Direction::East));
        let events = DataEffect::apply_all(
            &[DataEffect::LockExit {
                lock_exit: Direction::East,
            }],
            &mut world,
        );
        assert!(events.is_empty());
        assert!(world.is_exit_locked(Direction::East));
    }

    #[test]
    fn reveal_exit_effect_is_silent_and_unhides_the_door() {
        let mut world = world();
        assert!(world.is_exit_hidden(Direction::South));
        let events = DataEffect::apply_all(
            &[DataEffect::RevealExit {
                reveal_exit: Direction::South,
            }],
            &mut world,
        );
        assert!(events.is_empty());
        assert!(!world.is_exit_hidden(Direction::South));
    }

    #[test]
    fn hide_exit_effect_is_silent_and_hides_the_door() {
        let mut world = world();
        assert!(!world.is_exit_hidden(Direction::North));
        let events = DataEffect::apply_all(
            &[DataEffect::HideExit {
                hide_exit: Direction::North,
            }],
            &mut world,
        );
        assert!(events.is_empty());
        assert!(world.is_exit_hidden(Direction::North));
    }

    #[test]
    fn reveal_object_effect_is_silent_and_moves_object_to_visible() {
        let mut world = world();
        assert!(!world.room_object_names().contains(&"chest".to_string()));
        let events = DataEffect::apply_all(
            &[DataEffect::RevealObject {
                reveal_object: ObjectId::new("chest"),
            }],
            &mut world,
        );
        assert!(events.is_empty());
        assert!(world.room_object_names().contains(&"chest".to_string()));
    }

    #[test]
    fn hide_object_effect_is_silent_and_moves_object_to_hidden() {
        let mut world = world();
        assert!(world.room_object_names().contains(&"torch".to_string()));
        let events = DataEffect::apply_all(
            &[DataEffect::HideObject {
                hide_object: ObjectId::new("torch"),
            }],
            &mut world,
        );
        assert!(events.is_empty());
        assert!(!world.room_object_names().contains(&"torch".to_string()));
    }

    #[test]
    fn set_flag_and_clear_flag_effects_emit_and_mutate() {
        let mut world = world();
        let events = DataEffect::apply_all(
            &[
                DataEffect::SetFlag {
                    flag: "a".to_string(),
                },
                DataEffect::ClearFlag {
                    flag: "a".to_string(),
                },
            ],
            &mut world,
        );
        assert_eq!(
            events,
            vec![
                Event::FlagSet {
                    flag: "a".to_string()
                },
                Event::FlagCleared {
                    flag: "a".to_string()
                }
            ]
        );
        assert!(!world.has_flag("a"));
    }

    // -- InteractionData::matches / target_matches / condition_applies --

    fn base_interaction(verb: Verb) -> InteractionData {
        InteractionData {
            verb,
            item: None,
            target: None,
            condition: Vec::new(),
            effect: Vec::new(),
        }
    }

    #[test]
    fn interaction_data_matches_requires_verb_item_and_target() {
        let interaction = InteractionData {
            item: Some(ObjectId::new("sword")),
            target: Some(DataTarget::Object {
                object: ObjectId::new("cabinet"),
            }),
            ..base_interaction(Verb::Use)
        };
        let world = world();

        let matching = ActionContext::new(
            Some(Verb::Use),
            Some(ObjectId::new("sword")),
            Some(Target::Object(ObjectId::new("cabinet"))),
        );
        assert!(interaction.matches(&world, &matching));

        let wrong_verb = ActionContext::new(
            Some(Verb::Examine),
            Some(ObjectId::new("sword")),
            Some(Target::Object(ObjectId::new("cabinet"))),
        );
        assert!(!interaction.matches(&world, &wrong_verb));

        let wrong_item = ActionContext::new(
            Some(Verb::Use),
            Some(ObjectId::new("lamp")),
            Some(Target::Object(ObjectId::new("cabinet"))),
        );
        assert!(!interaction.matches(&world, &wrong_item));

        let wrong_target = ActionContext::new(
            Some(Verb::Use),
            Some(ObjectId::new("sword")),
            Some(Target::Object(ObjectId::new("sword"))),
        );
        assert!(!interaction.matches(&world, &wrong_target));
    }

    #[test]
    fn interaction_data_without_item_matches_any_item() {
        let interaction = base_interaction(Verb::Take);
        let world = world();
        let context = ActionContext::new(Some(Verb::Take), Some(ObjectId::new("sword")), None);
        assert!(interaction.matches(&world, &context));
    }

    #[test]
    fn interaction_data_without_target_requires_self_use() {
        let interaction = base_interaction(Verb::Use);
        let world = world();
        let self_use = ActionContext::new(Some(Verb::Use), None, None);
        let targeted = ActionContext::new(
            Some(Verb::Use),
            None,
            Some(Target::Object(ObjectId::new("sword"))),
        );
        assert!(interaction.matches(&world, &self_use));
        assert!(!interaction.matches(&world, &targeted));
    }

    #[test]
    fn interaction_data_npc_target_requires_exact_npc() {
        let interaction = InteractionData {
            target: Some(DataTarget::Npc {
                npc: crate::model::npc_id::NpcId::new("guard"),
            }),
            ..base_interaction(Verb::Use)
        };
        let world = world();
        let matching = ActionContext::new(
            Some(Verb::Use),
            None,
            Some(Target::Npc(crate::model::npc_id::NpcId::new("guard"))),
        );
        let other = ActionContext::new(
            Some(Verb::Use),
            None,
            Some(Target::Npc(crate::model::npc_id::NpcId::new("clerk"))),
        );
        assert!(interaction.matches(&world, &matching));
        assert!(!interaction.matches(&world, &other));
    }

    #[test]
    fn interaction_data_scene_kind_target_matches_any_scene_object() {
        let interaction = InteractionData {
            target: Some(DataTarget::Kind {
                kind: DataTargetKind::Scene,
            }),
            ..base_interaction(Verb::Use)
        };
        let world = world();
        let scene_target = ActionContext::new(
            Some(Verb::Use),
            None,
            Some(Target::Object(ObjectId::new("cabinet"))),
        );
        let item_target = ActionContext::new(
            Some(Verb::Use),
            None,
            Some(Target::Object(ObjectId::new("sword"))),
        );
        assert!(interaction.matches(&world, &scene_target));
        assert!(!interaction.matches(&world, &item_target));
    }

    #[test]
    fn interaction_data_conditions_must_all_hold() {
        let interaction = InteractionData {
            condition: vec![
                DataCondition::Room {
                    room: RoomId::new("room"),
                },
                DataCondition::Flag {
                    flag: "ready".to_string(),
                },
            ],
            ..base_interaction(Verb::Look)
        };
        let mut world = world();
        let context = no_target_context(Verb::Look);
        assert!(!interaction.matches(&world, &context));
        world.set_flag("ready");
        assert!(interaction.matches(&world, &context));
    }

    // -- InteractionData::run / compile --

    #[test]
    fn interaction_data_run_applies_its_effects_in_order() {
        let interaction = InteractionData {
            effect: vec![DataEffect::Take {
                take: ObjectId::new("sword"),
            }],
            ..base_interaction(Verb::Take)
        };
        let mut world = world();
        let events = interaction.run(&mut world);
        assert_eq!(
            events,
            vec![Event::Took {
                object_id: ObjectId::new("sword"),
                object: "sword".to_string()
            }]
        );
        assert!(world.player_holds(&ObjectId::new("sword")));
    }

    #[test]
    fn compile_reproduces_matches_and_runs_the_same_effects() {
        let data = InteractionData {
            item: Some(ObjectId::new("sword")),
            target: Some(DataTarget::Object {
                object: ObjectId::new("cabinet"),
            }),
            effect: vec![DataEffect::SetFlag {
                flag: "used-sword-on-cabinet".to_string(),
            }],
            ..base_interaction(Verb::Use)
        };
        let compiled = data.compile();
        let mut world = world();
        let context = ActionContext::new(
            Some(Verb::Use),
            Some(ObjectId::new("sword")),
            Some(Target::Object(ObjectId::new("cabinet"))),
        );
        assert!(compiled.matches(&world, &context));

        let non_matching_target = ActionContext::new(
            Some(Verb::Use),
            Some(ObjectId::new("sword")),
            Some(Target::Object(ObjectId::new("sword"))),
        );
        assert!(!compiled.matches(&world, &non_matching_target));

        let events = compiled.run(&mut world, &context);
        assert_eq!(
            events,
            vec![Event::FlagSet {
                flag: "used-sword-on-cabinet".to_string()
            }]
        );
        assert!(world.has_flag("used-sword-on-cabinet"));
    }

    // -- dispatch_data --

    #[test]
    fn dispatch_data_runs_the_first_matching_interaction() {
        let first = InteractionData {
            effect: vec![DataEffect::Emit {
                emit: "first".to_string(),
            }],
            ..base_interaction(Verb::Examine)
        };
        let second = InteractionData {
            effect: vec![DataEffect::Emit {
                emit: "second".to_string(),
            }],
            ..base_interaction(Verb::Examine)
        };
        let mut world = world_with_interactions(vec![first, second]);
        let context = no_target_context(Verb::Examine);
        assert_eq!(
            dispatch_data(&mut world, &context),
            Some(vec![Event::Custom {
                name: "first".to_string()
            }])
        );
    }

    #[test]
    fn dispatch_data_returns_none_when_nothing_matches() {
        let interaction = InteractionData {
            item: Some(ObjectId::new("sword")),
            ..base_interaction(Verb::Take)
        };
        let mut world = world_with_interactions(vec![interaction]);
        let context = no_target_context(Verb::Drop);
        assert_eq!(dispatch_data(&mut world, &context), None);
    }
}
