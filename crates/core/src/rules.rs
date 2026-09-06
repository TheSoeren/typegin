use crate::data::ObjectKind;
use crate::event;
use crate::input::action;
use crate::interaction::{ActionContext, Interaction, Verb};
use crate::world;
use crate::world::object::ObjectResolution;

/// Minimal rules that reuse every default hook.
///
/// Used when no custom rules are supplied to [`GameEngine::get`](crate::GameEngine::get).
pub struct BasicRules;

impl Rules for BasicRules {}

/// Hooks the game logic uses to decide behaviour.
///
/// This is how a game customizes rules: implement `Rules` and pass it to
/// [`GameEngine::get_with_rules`](crate::GameEngine::get_with_rules). The world
/// is passed in, so there is no wrapper boilerplate and no delegation. Every
/// method has a default implementation, so a custom type only implements the
/// hooks it wants to change.
///
/// Two complementary customization surfaces exist:
///
/// * **Per-verb defaults** — override one of the `on_*` hooks to change a
///   whole action category (`on_take`, `on_use`, ...).
/// * **Per-interaction rules** — provide [`Interaction`]s via [`Rules::interactions`].
///   These run *before* the default `on_use` fallback, so bespoke puzzle logic
///   ("cut the rope with the knife") authors as a single interaction instead of
///   a hook re-implementation. Front-ends can also enumerate them (see
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
    /// Only [`Item`](ObjectKind::Item) objects are portable — the default
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

        if let Some(ObjectKind::Scene) = world.object_kind(object_id) {
            return vec![event::Event::CantTake {
                object: name.to_string(),
            }];
        }

        let context = ActionContext::new(Some(Verb::Take), Some(object_id), None);
        if let Some(interaction) = self
            .interactions()
            .iter()
            .find(|interaction| interaction.matches(world, &context))
        {
            return interaction.run(world, &context);
        }

        match world.player_take_object(object_id) {
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

        let context = ActionContext::new(Some(Verb::Drop), Some(object_id), None);
        if let Some(interaction) = self
            .interactions()
            .iter()
            .find(|interaction| interaction.matches(world, &context))
        {
            return interaction.run(world, &context);
        }

        match world.player_drop_object(object_id) {
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
        resolution: ObjectResolution,
    ) -> Vec<event::Event> {
        let ObjectResolution::Found(object_id) = resolution else {
            return match resolution {
                ObjectResolution::Ambiguous { ids, alias } => {
                    vec![event::Event::ExaminedObjectAmbiguous {
                        object_ids: ids,
                        object: alias,
                    }]
                }
                _ => vec![event::Event::ExaminedObjectNotFound {
                    object: name.to_string(),
                }],
            };
        };

        let context = ActionContext::new(Some(Verb::Examine), Some(object_id), None);
        if let Some(interaction) = self
            .interactions()
            .iter()
            .find(|interaction| interaction.matches(world, &context))
        {
            return interaction.run(world, &context);
        }

        vec![event::Event::Examined {
            object_id,
            object: name.to_string(),
        }]
    }

    /// Decide what happens when the player uses an object, optionally on a target.
    ///
    /// The default runs three stages:
    ///
    /// 1. Match an authored [`Interaction`] (see [`Rules::interactions`]) — the
    ///    custom-puzzle slot.
    /// 2. Otherwise, fall back to the stock behaviour: a successful use on an
    ///    object emits `Used`; using an object on a *locked* door whose
    ///    `gated_by` is the used object unlocks it (`UnlockedExit`).
    /// 3. Everything else (wrong target, missing target, already open door)
    ///    gets a generic refusal event (`CannotUse`, etc.) — the fallback
    ///    spine that makes authored interactions cheap to write.
    fn on_use(
        &mut self,
        world: &mut world::WorldState,
        item: &str,
        target: Option<&str>,
        item_resolution: ObjectResolution,
        target_resolution: ObjectResolution,
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

        let target_id = match target_resolution {
            ObjectResolution::Found(id) => Some(id),
            _ => None,
        };
        let context = ActionContext::new(Some(Verb::Use), Some(item_id), target_id);
        if let Some(interaction) = self
            .interactions()
            .iter()
            .find(|interaction| interaction.matches(world, &context))
        {
            return interaction.run(world, &context);
        }

        let target_text = target.map(str::to_string);
        match target_resolution {
            ObjectResolution::Found(target_id) => {
                if let Some(direction) = world.exit_direction_of(target_id) {
                    // It's a door.
                    if world.is_exit_locked(direction)
                        && world.exit_gated_by(direction) == Some(item_id)
                    {
                        world.unlock_exit(direction);
                        vec![event::Event::UnlockedExit { direction }]
                    } else {
                        vec![event::Event::CannotUse {
                            item: item.to_string(),
                            target: target_text.unwrap_or_default(),
                        }]
                    }
                } else {
                    vec![event::Event::Used {
                        object_id: item_id,
                        object: item.to_string(),
                        target_id: Some(target_id),
                        target: target_text.clone(),
                    }]
                }
            }
            ObjectResolution::Ambiguous { ids, alias } => {
                vec![event::Event::UsedTargetAmbiguous {
                    object_id: item_id,
                    object: item.to_string(),
                    target_ids: ids,
                    target: alias,
                }]
            }
            ObjectResolution::NotFound => match target_text {
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

    /// Decide what happens for an unrecognised command.
    fn on_unknown(&mut self, _world: &mut world::WorldState, phrase: String) -> Vec<event::Event> {
        vec![event::Event::UnknownEvent { name: phrase }]
    }
}
