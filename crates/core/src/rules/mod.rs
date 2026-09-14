use std::collections::HashSet;

use crate::data::interactions_data::DataEffect;
use crate::input::{GoTarget, action};
use crate::interaction::{ActionContext, Interaction, Verb, dispatch_data};
use crate::world::object::{ObjectResolution, TargetResolution};
use crate::{Event, event, object_data};
use crate::{NpcId, ObjectId, Target, world};

/// Move the player through the door object `id` (a resolved target already
/// known to be present): locked, or entered.
///
/// Shared by [`Rules::on_go`]'s [`GoTarget::Named`] and [`GoTarget::Id`] arms
/// - both eventually reduce to "I have a door's id in scope, act on it,"
/// they just differ in how they got there (name resolution vs. a caller
/// supplying the id directly).
fn enter_door(world: &mut world::WorldState, id: ObjectId, name: String) -> Vec<event::Event> {
    if !world.object_is_door(&id) {
        return vec![event::Event::CantEnter { target: name }];
    }
    let door = world
        .object_info(&id)
        .and_then(|info| info.door)
        .expect("object_is_door confirmed door data is present");
    if door.locked {
        vec![event::Event::EnteredExitLocked {
            object_id: id,
            object: name,
        }]
    } else {
        match world.move_to_room(door.to) {
            action::Outcome::Success => vec![event::Event::Entered {
                object_id: id,
                object: name,
            }],
            action::Outcome::Fail => vec![event::Event::EnteredTargetNotFound { target: name }],
        }
    }
}

mod basic;

pub use basic::BasicRules;

/// Hooks the game logic uses to decide behaviour.
///
/// Implement `Rules` and pass it to
/// [`GameEngine::get_with_rules`](crate::GameEngine::get_with_rules); every
/// method has a default, so a custom type only overrides what it changes. Two
/// complementary customization surfaces exist:
///
/// * **Per-verb defaults** - override an `on_*` hook to change a whole action
///   category (`on_take`, `on_use`, ...).
/// * **Per-interaction rules** - provide [`Interaction`]s via
///   [`Rules::interactions`], run before the default `on_use` fallback, so
///   bespoke puzzle logic authors as one interaction instead of a hook
///   rewrite. Front-ends can also enumerate them (see
///   `GameEngine::interactions_for`) to build point-and-click menus.
pub trait Rules {
    /// Authored interactions, consulted before the default `on_use` fallback.
    ///
    /// The default returns none; provide interactions for custom puzzle logic.
    fn interactions(&self) -> &[Interaction] {
        &[]
    }

    /// Decide what happens when the player looks around the room.
    fn on_look(&mut self, _world: &mut world::WorldState) -> Vec<event::Event> {
        vec![event::Event::Looked]
    }

    /// Decide what happens when the player moves in a direction, enters a
    /// named/clicked exit, or enters a door referenced directly by id.
    ///
    /// `GoTarget::Direction` refuses a locked exit (`WentExitLocked`),
    /// reports a hidden one via `WentExitHidden` (how that reads to the
    /// player is the consumer's call), and otherwise follows the door.
    ///
    /// `GoTarget::Named` and `GoTarget::Id` reach doors that have no compass
    /// direction at all (or any door, by a point-and-click front-end that
    /// already has its id) - full name/id resolution against everything in
    /// scope (objects and NPCs), so entering an NPC or a non-door object is
    /// reported distinctly (`CantEnter`) from no such target at all.
    fn on_go(&mut self, world: &mut world::WorldState, target: GoTarget) -> Vec<event::Event> {
        match target {
            GoTarget::Direction(_) => {
                if world.is_exit_hidden(&target) {
                    vec![event::Event::WentExitHidden(target)]
                } else if world.is_exit_locked(&target) {
                    vec![event::Event::WentExitLocked(target)]
                } else {
                    match world.get_room_id_by_go_target(&target) {
                        Some(room_id) => match world.move_to_room(room_id) {
                            action::Outcome::Success => vec![event::Event::Went(target)],
                            action::Outcome::Fail => vec![event::Event::WentExitNotFound(target)],
                        },
                        None => vec![event::Event::WentExitNotFound(target)],
                    }
                }
            }
            GoTarget::Named(name) => match world.resolve_target(&name) {
                TargetResolution::Found(Target::Object(id)) => enter_door(world, id, name),
                TargetResolution::Found(Target::Npc(_)) => {
                    vec![event::Event::CantEnter { target: name }]
                }
                TargetResolution::Ambiguous { ids, alias } => {
                    vec![event::Event::EnteredTargetAmbiguous {
                        target_ids: ids,
                        target: alias,
                    }]
                }
                TargetResolution::NotFound => {
                    vec![event::Event::EnteredTargetNotFound { target: name }]
                }
            },
            GoTarget::Id(id) => {
                let name = world.object_display_name(&id);
                if world.target_in_scope(&id) {
                    enter_door(world, id, name)
                } else {
                    vec![event::Event::EnteredTargetNotFound { target: name }]
                }
            }
        }
    }

    /// Decide what happens when the player tries to take an object.
    ///
    /// Only [`Item`](crate::object_data::ObjectKind::Item) objects are portable - the default
    /// takes them into inventory. Scene objects (furniture, doors, ...) are a
    /// fixed part of the world and are refused with `CantTake`; authored
    /// interactions never get a say here (use flows through `on_use`).
    fn on_take(
        &mut self,
        world: &mut world::WorldState,
        name: &str,
        resolution: ObjectResolution,
    ) -> Vec<event::Event> {
        let ObjectResolution::Found(object_id) = resolution else {
            return match resolution {
                ObjectResolution::Ambiguous { ids, alias } => {
                    vec![event::Event::TookObjectAmbiguous {
                        object_ids: ids,
                        object: alias,
                    }]
                }
                _ => vec![event::Event::TookObjectNotFound {
                    object: name.to_string(),
                }],
            };
        };

        if let Some(object_data::ObjectKind::Scene) = world.object_kind(&object_id) {
            return vec![event::Event::CantTake {
                object: name.to_string(),
            }];
        }

        let context = ActionContext::new(Some(Verb::Take), Some(object_id.clone()), None);
        if let Some(events) = dispatch_data(world, &context) {
            return events;
        }
        if let Some(interaction) = self
            .interactions()
            .iter()
            .find(|interaction| interaction.matches(world, &context))
        {
            return interaction.run(world, &context);
        }

        match world.player_take_object(&object_id) {
            action::Outcome::Success => vec![event::Event::Took {
                object_id,
                object: name.to_string(),
            }],
            action::Outcome::Fail => {
                vec![event::Event::TookObjectNotFound {
                    object: name.to_string(),
                }]
            }
        }
    }

    /// Decide what happens when the player tries to drop an object.
    fn on_drop(
        &mut self,
        world: &mut world::WorldState,
        name: &str,
        resolution: ObjectResolution,
    ) -> Vec<event::Event> {
        let ObjectResolution::Found(object_id) = resolution else {
            return match resolution {
                ObjectResolution::Ambiguous { ids, alias } => {
                    vec![event::Event::DroppedObjectAmbiguous {
                        object_ids: ids,
                        object: alias,
                    }]
                }
                _ => vec![event::Event::DroppedObjectNotFound {
                    object: name.to_string(),
                }],
            };
        };

        let context = ActionContext::new(Some(Verb::Drop), Some(object_id.clone()), None);
        if let Some(events) = dispatch_data(world, &context) {
            return events;
        }
        if let Some(interaction) = self
            .interactions()
            .iter()
            .find(|interaction| interaction.matches(world, &context))
        {
            return interaction.run(world, &context);
        }

        match world.player_drop_object(&object_id) {
            action::Outcome::Success => vec![event::Event::Dropped {
                object_id,
                object: name.to_string(),
            }],
            action::Outcome::Fail => vec![event::Event::DroppedObjectNotFound {
                object: name.to_string(),
            }],
        }
    }

    /// Decide what happens when the player examines a thing.
    ///
    /// Any object in scope - carried, in the room, or a door - can be examined.
    fn on_examine(
        &mut self,
        world: &mut world::WorldState,
        name: &str,
        resolution: TargetResolution,
    ) -> Vec<event::Event> {
        let TargetResolution::Found(target) = resolution else {
            return match resolution {
                TargetResolution::Ambiguous { ids, alias } => {
                    vec![event::Event::ExaminedTargetAmbiguous {
                        target_ids: ids,
                        target: alias,
                    }]
                }
                _ => vec![event::Event::ExaminedTargetNotFound {
                    target: name.to_string(),
                }],
            };
        };

        let context = ActionContext::new(Some(Verb::Examine), None, Some(target.clone()));
        if let Some(events) = dispatch_data(world, &context) {
            return events;
        }
        if let Some(interaction) = self
            .interactions()
            .iter()
            .find(|interaction| interaction.matches(world, &context))
        {
            return interaction.run(world, &context);
        }

        vec![event::Event::Examined {
            target,
            target_name: name.to_string(),
        }]
    }

    /// Decide what happens when the player uses an object, optionally on a target.
    fn on_use(
        &mut self,
        world: &mut world::WorldState,
        item: &str,
        target: Option<&str>,
        item_resolution: ObjectResolution,
        target_resolution: TargetResolution,
    ) -> Vec<event::Event> {
        let ObjectResolution::Found(item_id) = item_resolution else {
            return match item_resolution {
                ObjectResolution::Ambiguous { ids, alias } => {
                    vec![event::Event::UsedObjectAmbiguous {
                        object_ids: ids,
                        object: alias,
                    }]
                }
                _ => vec![event::Event::UsedObjectNotFound {
                    object: item.to_string(),
                }],
            };
        };

        let target_id = match &target_resolution {
            TargetResolution::Found(t) => Some(t.clone()),
            _ => None,
        };
        let context = ActionContext::new(Some(Verb::Use), Some(item_id.clone()), target_id);
        if let Some(events) = dispatch_data(world, &context) {
            return events;
        }
        if let Some(interaction) = self
            .interactions()
            .iter()
            .find(|interaction| interaction.matches(world, &context))
        {
            return interaction.run(world, &context);
        }

        let target_text = target.map(str::to_string);
        match target_resolution {
            TargetResolution::Found(target_id) => {
                vec![event::Event::Used {
                    object_id: item_id,
                    object: item.to_string(),
                    target_id: Some(target_id),
                    target: target_text.clone(),
                }]
            }
            TargetResolution::Ambiguous { ids, alias } => {
                vec![event::Event::UsedTargetAmbiguous {
                    object_id: item_id,
                    object: item.to_string(),
                    target_ids: ids,
                    target: alias,
                }]
            }
            TargetResolution::NotFound => match target_text {
                None => vec![event::Event::UsedTargetNeeded {
                    object_id: item_id,
                    object: item.to_string(),
                }],
                Some(target) => vec![event::Event::UsedTargetNotFound {
                    object_id: item_id,
                    object: item.to_string(),
                    target,
                }],
            },
        }
    }

    /// Decide what happens when the player talks to an NPC (already resolved
    /// to `npc_id` by name or by a caller-supplied id; `name` is the display
    /// name to attribute to a not-found event).
    ///
    /// Start (or advance) the NPC's conversation: show the current dialogue
    /// node, and end the conversation when the node has no choices.
    fn on_talk(
        &mut self,
        world: &mut world::WorldState,
        name: &str,
        npc_id: Option<NpcId>,
    ) -> Vec<event::Event> {
        let Some(npc) = npc_id.and_then(|id| world.npcs().iter().find(|npc| npc.id() == &id))
        else {
            return vec![Event::TalkNpcNotFound {
                npc: name.to_string(),
            }];
        };

        let node_id = world
            .npc_dialogue_node(&npc.id)
            .cloned()
            .unwrap_or_else(|| npc.root.clone());
        let Some(talk_event) = Event::talked(npc, &node_id) else {
            return Vec::new();
        };
        let ended = npc
            .dialogue_node(&node_id)
            .is_some_and(|node| node.choices().is_empty());
        let ended_event = ended.then(|| Event::dialogue_ended(npc));
        let npc_id = npc.id.clone();

        world.set_active_npc(npc_id.clone());
        world.set_dialogue_node(npc_id.clone(), node_id);

        let mut events = vec![talk_event];
        if let Some(ended_event) = ended_event {
            events.push(ended_event);
            world.clear_dialogue(&npc_id);
            world.clear_active_npc();
        }
        events
    }

    /// Decide what happens when the player picks a dialogue option.
    ///
    /// Match the choice on the active conversation by 1-based index or
    /// case-insensitive label, run its effects, and advance to the next node
    /// (or end). `UnknownEvent` when no conversation is active, and
    /// `DialogueInvalidChoice` when nothing matches.
    fn on_choose(&mut self, world: &mut world::WorldState, choice: &str) -> Vec<event::Event> {
        let Some(npc_id) = world.active_npc().clone() else {
            return vec![Event::UnknownEvent {
                name: choice.to_string(),
            }];
        };
        let Some(current_node_id) = world.npc_dialogue_node(&npc_id).cloned() else {
            return vec![Event::UnknownEvent {
                name: choice.to_string(),
            }];
        };

        // Resolve the current node and chosen option read-only, so the effects
        // can then mutate the world freely.
        let Some(npc) = world.npcs().iter().find(|npc| npc.id() == &npc_id) else {
            return vec![Event::UnknownEvent {
                name: choice.to_string(),
            }];
        };
        let npc_name = npc.primary_name().to_string();
        let Some(chosen) = npc
            .dialogue_node(&current_node_id)
            .and_then(|node| node.find_choice(choice))
        else {
            return vec![Event::DialogueInvalidChoice {
                npc: npc_name,
                choice: choice.to_string(),
            }];
        };

        let next_node_id = chosen.next().cloned();
        let effects = chosen.effect().to_vec();
        let next_event = next_node_id
            .as_ref()
            .and_then(|next| Event::talked(npc, next));
        let ended = match &next_node_id {
            None => true,
            Some(next) => npc
                .dialogue_node(next)
                .is_some_and(|node| node.choices().is_empty()),
        };

        let mut events = DataEffect::apply_all(&effects, world);

        if let Some(next) = next_node_id {
            if let Some(next_event) = next_event {
                events.push(next_event);
            }
            if ended {
                events.push(Event::DialogueEnded {
                    npc_id: npc_id.clone(),
                    npc: npc_name,
                });
                world.clear_dialogue(&npc_id);
                world.clear_active_npc();
            } else {
                world.set_dialogue_node(npc_id, next);
            }
        } else {
            events.push(Event::DialogueEnded {
                npc_id: npc_id.clone(),
                npc: npc_name,
            });
            world.clear_dialogue(&npc_id);
            world.clear_active_npc();
        }
        events
    }

    /// Decide what happens for an unrecognised command.
    fn on_unknown(&mut self, _world: &mut world::WorldState, phrase: String) -> Vec<event::Event> {
        vec![event::Event::UnknownEvent { name: phrase }]
    }

    /// Decide the final set of verbs `GameEngine::verbs_for` reports for
    /// `target`, given `interaction_verbs` (every verb a currently-live
    /// item-agnostic interaction reports for it). This hook has the final
    /// say - whatever it returns *is* the coin's contents, so an override
    /// that wants to keep `interaction_verbs` around must fold them back in
    /// itself.
    ///
    /// The default mirrors what the other stock `on_*` hooks would actually
    /// do, so the coin never claims a verb the engine would then refuse (or
    /// omit one it would honour):
    ///
    /// * An open (unlocked) exit collapses the coin to `Go` alone - no
    ///   verb-coin at all, matching the "just a walk cursor" convention
    ///   modern point-and-click adventures use for an exit that needs no
    ///   further interaction - deliberately discarding `interaction_verbs`
    ///   to keep it that way. A consumer wanting an authored interaction to
    ///   still show up alongside `Go` on an open door overrides this hook
    ///   and unions `interaction_verbs` in itself.
    /// * A non-`Scene` object currently in the room gets `Take` (mirrors
    ///   `on_take`'s only gate: `Scene` objects refuse with `CantTake`).
    /// * An object currently in inventory gets `Drop` (mirrors `on_drop`,
    ///   which only succeeds for a carried object).
    /// * Everything else (a locked exit, an NPC, ...) just gets the ordinary
    ///   default: `interaction_verbs` plus `Examine`.
    fn verbs_for(
        &self,
        target: Target,
        interaction_verbs: &HashSet<Verb>,
        world: &world::WorldState,
    ) -> HashSet<Verb> {
        let Target::Object(id) = &target else {
            let mut verbs = interaction_verbs.clone();
            verbs.insert(Verb::Examine);
            return verbs;
        };

        let is_open_exit = world
            .object_info(id)
            .and_then(|info| info.door)
            .is_some_and(|door| !door.locked);
        if is_open_exit {
            return HashSet::from([Verb::Go]);
        }

        let mut verbs = interaction_verbs.clone();
        verbs.insert(Verb::Examine);
        if !world.object_is_scene(id)
            && matches!(world.get_object_from_room(id), ObjectResolution::Found(_))
        {
            verbs.insert(Verb::Take);
        }
        if world.player_holds(id) {
            verbs.insert(Verb::Drop);
        }
        verbs
    }
}
