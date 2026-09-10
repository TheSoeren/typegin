use crate::data::interactions_data::DataEffect;
use crate::input::action;
use crate::interaction::{ActionContext, Interaction, Verb, dispatch_data};
use crate::world;
use crate::world::object::{ObjectResolution, TargetResolution};
use crate::{Event, event, object_data};

mod basic;

pub use basic::BasicRules;

/// Hooks the game logic uses to decide behaviour.
///
/// Implement `Rules` and pass it to
/// [`GameEngine::get_with_rules`](crate::GameEngine::get_with_rules); every
/// method has a default, so a custom type only overrides what it changes. Two
/// complementary customization surfaces exist:
///
/// * **Per-verb defaults** — override an `on_*` hook to change a whole action
///   category (`on_take`, `on_use`, ...).
/// * **Per-interaction rules** — provide [`Interaction`]s via
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

    /// Decide what happens when the player moves in a direction.
    ///
    /// The default refuses a locked exit (`WentExitLocked`), reports a hidden
    /// one via `WentExitHidden` (how that reads to the player is the
    /// consumer's call), and otherwise follows the door.
    fn on_go(
        &mut self,
        world: &mut world::WorldState,
        direction: crate::input::Direction,
    ) -> Vec<event::Event> {
        if world.is_exit_hidden(direction) {
            vec![event::Event::WentExitHidden(direction)]
        } else if world.is_exit_locked(direction) {
            vec![event::Event::WentExitLocked(direction)]
        } else {
            match world.get_room_id_by_exit_direction(direction) {
                Some(room_id) => match world.move_to_room(room_id) {
                    action::MoveResult::Success => vec![event::Event::Went(direction)],
                    action::MoveResult::Fail => vec![event::Event::WentInvalidDirection(direction)],
                },
                None => vec![event::Event::WentInvalidDirection(direction)],
            }
        }
    }

    /// Decide what happens when the player tries to take an object.
    ///
    /// Only [`Item`](crate::object_data::ObjectKind::Item) objects are portable — the default
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
            action::TakeResult::Success => vec![event::Event::Took {
                object_id,
                object: name.to_string(),
            }],
            action::TakeResult::Fail => {
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
            action::DropResult::Success => vec![event::Event::Dropped {
                object_id,
                object: name.to_string(),
            }],
            action::DropResult::Fail => vec![event::Event::DroppedObjectNotFound {
                object: name.to_string(),
            }],
        }
    }

    /// Decide what happens when the player examines a thing.
    ///
    /// Any object in scope — carried, in the room, or a door — can be examined.
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

    /// Decide what happens when the player talks to an NPC by name.
    ///
    /// Start (or advance) the NPC's conversation: show the current dialogue
    /// node, and end the conversation when the node has no choices.
    fn on_talk(&mut self, world: &mut world::WorldState, name: &str) -> Vec<event::Event> {
        let Some(npc) = world.resolve_npc(name) else {
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
}
