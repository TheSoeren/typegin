//! Integration tests for the default `Rules` implementations.
//!
//! `GameEngine::open` uses `BasicRules`, so these tests exercise every default
//! hook through the public `handle_input` API.
//!
//! Run with: cd crates/core && cargo test --test default_rules

mod common;

use common::setup_engine;
use core::{Direction, Event, RoomId};

// --- on_look ---

mod on_look {
    use super::*;

    #[test]
    fn look_emits_looked() {
        let mut engine = setup_engine();
        assert_eq!(engine.handle_input("look"), vec![Event::Looked]);
    }

    #[test]
    fn l_shortcut_emits_looked() {
        let mut engine = setup_engine();
        assert_eq!(engine.handle_input("l"), vec![Event::Looked]);
    }
}

// --- on_go ---

mod on_go {
    use super::*;

    #[test]
    fn valid_exit_moves_rooms() {
        let mut engine = setup_engine();
        assert_eq!(
            engine.handle_input("go north"),
            vec![Event::Went(Direction::North)]
        );
        assert_eq!(engine.world.current_room_id(), RoomId::new(2));
    }

    #[test]
    fn single_direction_word_moves() {
        let mut engine = setup_engine();
        // Room 1 only has a north exit; a bare direction word is accepted.
        assert_eq!(
            engine.handle_input("north"),
            vec![Event::Went(Direction::North)]
        );
        assert_eq!(engine.world.current_room_id(), RoomId::new(2));
    }

    #[test]
    fn invalid_exit_reports_invalid_direction() {
        let mut engine = setup_engine();
        // Room 1 only has a north exit.
        assert_eq!(
            engine.handle_input("go east"),
            vec![Event::WentInvalidDirection(Direction::East)]
        );
        assert_eq!(engine.world.current_room_id(), RoomId::new(1));
    }

    #[test]
    fn same_direction_event_from_other_room() {
        let mut engine = setup_engine();
        engine.handle_input("go north"); // now in room 2
        // Room 2 has south and east exits, not west.
        assert_eq!(
            engine.handle_input("go west"),
            vec![Event::WentInvalidDirection(Direction::West)]
        );
    }
}

// --- on_take ---

mod on_take {
    use super::*;

    #[test]
    fn take_item_moves_to_inventory() {
        let mut engine = setup_engine();
        assert_eq!(
            engine.handle_input("take iron key"),
            vec![Event::Took {
                item: "iron key".to_string()
            }]
        );
        assert!(
            engine.world.get_item_from_room(core::ItemId::new(2)) == core::ItemResolution::NotFound
        );
        assert!(
            engine.world.get_item_from_player(core::ItemId::new(2))
                != core::ItemResolution::NotFound
        );
    }

    #[test]
    fn take_unknown_item_reports_not_found() {
        let mut engine = setup_engine();
        assert_eq!(
            engine.handle_input("take bogus"),
            vec![Event::TookItemNotFound {
                item: "bogus".to_string()
            }]
        );
    }

    #[test]
    fn take_ambiguous_name_reports_ambiguous() {
        let mut engine = setup_engine();
        // "key" matches both item 2 (iron key) and item 4 (brass key) in room 1.
        assert_eq!(
            engine.handle_input("take key"),
            vec![Event::TookItemAmbiguous {
                item: "key".to_string()
            }]
        );
    }

    #[test]
    fn take_already_holding_item_reports_not_found() {
        let mut engine = setup_engine();
        engine.handle_input("take iron key");
        // The item is no longer in the room, so a second take matches nothing.
        assert_eq!(
            engine.handle_input("take iron key"),
            vec![Event::TookItemNotFound {
                item: "iron key".to_string()
            }]
        );
    }
}

// --- on_drop ---

mod on_drop {
    use super::*;

    #[test]
    fn drop_item_moves_to_room() {
        let mut engine = setup_engine();
        engine.handle_input("take iron key");
        assert_eq!(
            engine.handle_input("drop iron key"),
            vec![Event::Dropped {
                item: "iron key".to_string()
            }]
        );
        assert!(
            engine.world.get_item_from_player(core::ItemId::new(2))
                == core::ItemResolution::NotFound
        );
        assert!(
            engine.world.get_item_from_room(core::ItemId::new(2)) != core::ItemResolution::NotFound
        );
    }

    #[test]
    fn drop_item_not_held_reports_not_found() {
        let mut engine = setup_engine();
        assert_eq!(
            engine.handle_input("drop iron key"),
            vec![Event::DroppedItemNotFound {
                item: "iron key".to_string()
            }]
        );
    }

    #[test]
    fn drop_ambiguous_item_reports_ambiguous() {
        let mut engine = setup_engine();
        engine.handle_input("take iron key");
        engine.handle_input("take brass key");
        assert_eq!(
            engine.handle_input("drop key"),
            vec![Event::DroppedItemAmbiguous {
                item: "key".to_string()
            }]
        );
    }
}

// --- on_examine ---

mod on_examine {
    use super::*;

    #[test]
    fn examine_found_item_emits_examined() {
        let mut engine = setup_engine();
        assert_eq!(
            engine.handle_input("examine iron key"),
            vec![Event::Examined {
                item: "iron key".to_string()
            }]
        );
    }

    #[test]
    fn examine_unknown_item_reports_not_found() {
        let mut engine = setup_engine();
        assert_eq!(
            engine.handle_input("examine bogus"),
            vec![Event::ExaminedItemNotFound {
                item: "bogus".to_string()
            }]
        );
    }

    #[test]
    fn examine_ambiguous_item_reports_ambiguous() {
        let mut engine = setup_engine();
        assert_eq!(
            engine.handle_input("examine key"),
            vec![Event::ExaminedItemAmbiguous {
                item: "key".to_string()
            }]
        );
    }
}

// --- on_use ---

mod on_use {
    use super::*;

    #[test]
    fn use_unheld_item_reports_item_not_found() {
        let mut engine = setup_engine();
        assert_eq!(
            engine.handle_input("use sword"),
            vec![Event::UsedItemNotFound {
                item: "sword".to_string()
            }]
        );
    }

    #[test]
    fn use_held_item_without_target() {
        let mut engine = setup_engine();
        engine.handle_input("take iron key");
        assert_eq!(
            engine.handle_input("use iron key"),
            vec![Event::UsedTargetNeeded {
                item: "iron key".to_string()
            }]
        );
    }

    #[test]
    fn use_held_item_with_target() {
        let mut engine = setup_engine();
        engine.handle_input("take iron key");
        assert_eq!(
            engine.handle_input("use iron key on chest"),
            vec![Event::Used {
                item: "iron key".to_string(),
                target: Some("chest".to_string()),
            }]
        );
    }

    #[test]
    fn use_ambiguous_item_reports_ambiguous() {
        let mut engine = setup_engine();
        engine.handle_input("take iron key");
        engine.handle_input("take brass key");
        // The player holds both keys, so "key" is ambiguous.
        assert_eq!(
            engine.handle_input("use key on chest"),
            vec![Event::UsedItemAmbiguous {
                item: "key".to_string()
            }]
        );
    }
}

// --- on_unknown ---

mod on_unknown {
    use super::*;

    #[test]
    fn unknown_command_reports_unknown_event() {
        let mut engine = setup_engine();
        assert_eq!(
            engine.handle_input("dance wildly"),
            vec![Event::UnknownEvent {
                name: "dance wildly".to_string()
            }]
        );
    }

    #[test]
    fn unknown_direction_reports_unknown_event() {
        let mut engine = setup_engine();
        assert_eq!(
            engine.handle_input("go sideways"),
            vec![Event::UnknownEvent {
                name: "go sideways".to_string()
            }]
        );
    }
}
