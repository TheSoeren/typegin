use crate::event;
use crate::input::action;
use crate::world;
use crate::world::item;

/// Minimal rules that reuse every default hook.
///
/// Used when no custom rules are supplied to [`GameEngine::open`].
pub struct BasicRules;

impl Rules for BasicRules {}

/// Hooks the game logic uses to decide behaviour.
///
/// This is how a game customizes rules: implement `Rules` and pass it to
/// [`GameEngine::get_with_rules`]. The world is passed in, so there is no
/// wrapper boilerplate and no delegation. Every method has a default
/// implementation, so a custom type only implements the hooks it wants to
/// change.
///
/// For actions that reference world entities (`take`, `examine`, `use`), the
/// engine resolves the name before calling the hook and passes both the
/// resulting [`Resolution`] and the ability to look up the matched
/// [`ItemInfo`] from the world — so a rule can act on the real entity (its
/// id, name and aliases) rather than a raw name string.
pub trait Rules {
    /// Decide what happens when the player looks around the room.
    fn on_look(&mut self, _world: &mut world::WorldState) -> Vec<event::Event> {
        vec![event::Event::Looked]
    }

    /// Decide what happens when the player moves in a direction.
    ///
    /// The default follows the exit, emitting nothing if there is none.
    fn on_go(
        &mut self,
        world: &mut world::WorldState,
        direction: crate::input::Direction,
    ) -> Vec<event::Event> {
        match world.get_room_id_by_exit_direction(direction) {
            Some(room_id) => {
                world.move_to_room(room_id);
                vec![event::Event::Went(direction)]
            }
            None => vec![event::Event::WentInvalidDirection(direction)],
        }
    }

    /// Decide what happens when the player tries to take an item.
    fn on_take(
        &mut self,
        world: &mut world::WorldState,
        name: &str,
        resolution: item::ItemResolution,
    ) -> Vec<event::Event> {
        match resolution {
            item::ItemResolution::Found(id) => match world.player_take_item(id) {
                action::TakeResult::Success => vec![event::Event::Took {
                    item: name.to_string(),
                }],
                action::TakeResult::Fail => {
                    vec![event::Event::TookItemNotFound {
                        item: name.to_string(),
                    }]
                }
            },
            item::ItemResolution::Ambiguous(_) => vec![event::Event::TookItemAmbiguous {
                item: name.to_string(),
            }],
            item::ItemResolution::NotFound => vec![event::Event::TookItemNotFound {
                item: name.to_string(),
            }],
        }
    }

    /// Decide what happens when the player tries to drop an item.
    fn on_drop(
        &mut self,
        world: &mut world::WorldState,
        name: &str,
        resolution: item::ItemResolution,
    ) -> Vec<event::Event> {
        match resolution {
            item::ItemResolution::Found(id) => match world.player_drop_item(id) {
                action::DropResult::Success => vec![event::Event::Dropped {
                    item: name.to_string(),
                }],
                action::DropResult::Fail => {
                    vec![event::Event::DroppedItemNotFound {
                        item: name.to_string(),
                    }]
                }
            },
            item::ItemResolution::Ambiguous(_) => vec![event::Event::DroppedItemAmbiguous {
                item: name.to_string(),
            }],
            item::ItemResolution::NotFound => vec![event::Event::DroppedItemNotFound {
                item: name.to_string(),
            }],
        }
    }

    /// Decide what happens when the player examines a thing.
    fn on_examine(
        &mut self,
        _world: &mut world::WorldState,
        name: &str,
        resolution: item::ItemResolution,
    ) -> Vec<event::Event> {
        match resolution {
            item::ItemResolution::Found(_) => {
                vec![event::Event::Examined {
                    item: name.to_string(),
                }]
            }
            item::ItemResolution::Ambiguous(_) => {
                vec![event::Event::ExaminedItemAmbiguous {
                    item: name.to_string(),
                }]
            }
            item::ItemResolution::NotFound => {
                vec![event::Event::ExaminedItemNotFound {
                    item: name.to_string(),
                }]
            }
        }
    }

    /// Decide what happens when the player uses an item on a target.
    fn on_use(
        &mut self,
        _world: &mut world::WorldState,
        item: &str,
        target: Option<&str>,
        item_resolution: item::ItemResolution,
        target_resolution: item::ItemResolution,
    ) -> Vec<event::Event> {
        match item_resolution {
            item::ItemResolution::Found(_) => match target_resolution {
                item::ItemResolution::Found(_) => {
                    vec![event::Event::Used {
                        item: item.to_string(),
                        target: target.map(String::from),
                    }]
                }
                item::ItemResolution::Ambiguous(_) => {
                    vec![event::Event::UsedItemAmbiguous {
                        item: item.to_string(),
                    }]
                }
                item::ItemResolution::NotFound => {
                    vec![event::Event::UsedTargetNeeded {
                        item: item.to_string(),
                    }]
                }
            },
            item::ItemResolution::Ambiguous(_) => {
                vec![event::Event::UsedItemAmbiguous {
                    item: item.to_string(),
                }]
            }
            item::ItemResolution::NotFound => {
                vec![event::Event::UsedItemNotFound {
                    item: item.to_string(),
                }]
            }
        }
    }

    /// Decide what happens for an unrecognised command.
    fn on_unknown(&mut self, _world: &mut world::WorldState, phrase: String) -> Vec<event::Event> {
        vec![event::Event::UnknownEvent { name: phrase }]
    }
}
