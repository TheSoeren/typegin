//! Spec (red phase) for the opaque `extra` carrier.
//!
//! Game- and front-end-specific data (sprites, weights, puzzle hints, ...)
//! travels through the engine untouched. It is declared as a TOML table on an
//! item or room, mirrored on the public `ItemInfo`/`WorldState`, and readable
//! by custom `Rules` — without the engine ever interpreting the keys. Key
//! namespacing (`gui.*`, `mechanics.*`) is a convention, not engine knowledge.

mod common;

use std::collections::HashMap;

use core::data::ExtraValue;
use core::event::Event;
use core::{GameEngine, ItemId, Rules, WorldState};

fn item_2_extra() -> HashMap<String, ExtraValue> {
    let mut extra = HashMap::new();
    let mut gui = HashMap::new();
    gui.insert(
        "icon".to_string(),
        ExtraValue::Str("sprites/key.png".to_string()),
    );
    extra.insert("gui".to_string(), ExtraValue::Table(gui));
    extra.insert("weight".to_string(), ExtraValue::Int(3));
    extra.insert("opens".to_string(), ExtraValue::Str("chest".to_string()));
    extra.insert(
        "materials".to_string(),
        ExtraValue::Array(vec![
            ExtraValue::Str("metal".to_string()),
            ExtraValue::Str("iron".to_string()),
        ]),
    );
    let mut tags = HashMap::new();
    tags.insert("kind".to_string(), ExtraValue::Str("key".to_string()));
    extra.insert("tags".to_string(), ExtraValue::Table(tags));
    extra
}

fn room_1_extra() -> HashMap<String, ExtraValue> {
    let mut extra = HashMap::new();
    extra.insert("dark".to_string(), ExtraValue::Bool(true));
    let mut gui = HashMap::new();
    gui.insert(
        "background".to_string(),
        ExtraValue::Str("bg/cellar.png".to_string()),
    );
    extra.insert("gui".to_string(), ExtraValue::Table(gui));
    extra
}

// --- TOML data parsing ---

mod data_parsing {
    use super::*;

    #[test]
    fn item_extra_parses_all_types() {
        let data = common::multi_room_world_data();
        let key = data.find_item(2).expect("item 2 exists");
        assert_eq!(key.extra, item_2_extra());
    }

    #[test]
    fn room_extra_parses() {
        let data = common::multi_room_world_data();
        let room = data.find_room(1).expect("room 1 exists");
        assert_eq!(room.extra, room_1_extra());
    }

    #[test]
    fn item_without_extra_parses_as_empty() {
        let data = common::multi_room_world_data();
        let sword = data.find_item(1).expect("item 1 exists");
        assert!(sword.extra.is_empty());
    }

    #[test]
    fn room_without_extra_parses_as_empty() {
        let data = common::multi_room_world_data();
        let corridor = data.find_room(2).expect("room 2 exists");
        assert!(corridor.extra.is_empty());
    }
}

// --- World state exposure ---

mod world_exposure {
    use super::*;

    #[test]
    fn item_info_exposes_extra() {
        let engine = common::setup_engine();
        let info = engine
            .world()
            .item_info(ItemId::new(2))
            .expect("item 2 in room");
        assert_eq!(info.extra, item_2_extra());
    }

    #[test]
    fn item_without_extra_exposes_empty_map() {
        let engine = common::setup_engine();
        let info = engine
            .world()
            .item_info(ItemId::new(3))
            .expect("item 3 in room");
        assert!(info.extra.is_empty());
    }

    #[test]
    fn current_room_extra_matches_room_data() {
        let engine = common::setup_engine();
        assert_eq!(engine.world().current_room_extra(), room_1_extra());
    }

    #[test]
    fn other_room_has_no_extra() {
        let mut engine = common::setup_engine();
        engine.handle_input("go north");
        assert!(engine.world().current_room_extra().is_empty());
    }
}

// --- Rules can read extra data without engine knowledge ---

/// A rule that vetoes taking anything whose `extra.weight` is over 2, leaving
/// the item in the room. Items without a weight are unaffected.
struct VetoHeavyRules;

fn weight_of(world: &WorldState, id: ItemId) -> Option<i64> {
    world
        .item_info(id)
        .and_then(|info| match info.extra.get("weight") {
            Some(ExtraValue::Int(kg)) => Some(*kg),
            _ => None,
        })
}

impl Rules for VetoHeavyRules {
    fn on_take(
        &mut self,
        world: &mut WorldState,
        name: &str,
        resolution: core::ItemResolution,
    ) -> Vec<Event> {
        match resolution {
            core::ItemResolution::Found(id) => {
                if weight_of(world, id).is_some_and(|kg| kg > 2) {
                    Vec::new()
                } else {
                    match world.player_take_item(id) {
                        core::TakeResult::Success => vec![Event::Took {
                            item_id: id,
                            item: name.to_string(),
                        }],
                        core::TakeResult::Fail => {
                            vec![Event::TookItemNotFound {
                                item: name.to_string(),
                            }]
                        }
                    }
                }
            }
            core::ItemResolution::Ambiguous { ids, alias } => vec![Event::TookItemAmbiguous {
                item_ids: ids,
                item: alias,
            }],
            core::ItemResolution::NotFound => vec![Event::TookItemNotFound {
                item: name.to_string(),
            }],
        }
    }
}

mod rules_read_extra {
    use super::*;

    fn veto_engine() -> GameEngine {
        common::setup_engine_with_rules(VetoHeavyRules)
    }

    #[test]
    fn heavy_item_is_vetoed_and_stays_in_room() {
        let mut engine = veto_engine();
        assert!(engine.handle_input("take iron key").is_empty());
        assert!(
            !engine
                .world()
                .player_item_names()
                .contains(&"iron key".to_string())
        );
        assert!(
            engine
                .world()
                .room_item_names()
                .contains(&"iron key".to_string())
        );
    }

    #[test]
    fn weightless_item_is_taken_normally() {
        let mut engine = veto_engine();
        assert_eq!(
            engine.handle_input("take brass key"),
            vec![Event::Took {
                item_id: ItemId::new(4),
                item: "brass key".to_string()
            }]
        );
        assert!(
            engine
                .world()
                .player_item_names()
                .contains(&"brass key".to_string())
        );
    }
}
