//! Integration tests for the default `Rules` implementations.
//!
//! `GameEngine::open` uses `BasicRules`, so these tests exercise every default
//! hook through the public `handle_input` API.
//!
//! Run with: cd crates/core && cargo test --test default_rules

mod common;

use core::{Direction, Event};
use common::setup_engine;

fn messages(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Message(text) => Some(text.clone()),
            _ => None,
        })
        .collect()
}

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
        assert_eq!(engine.handle_input("go north"), vec![Event::Went(Direction::North)]);
        assert_eq!(engine.world.current_room_id(), 2);
    }

    #[test]
    fn single_direction_word_moves() {
        let mut engine = setup_engine();
        // Room 1 only has a north exit; a bare direction word is accepted.
        assert_eq!(engine.handle_input("north"), vec![Event::Went(Direction::North)]);
        assert_eq!(engine.world.current_room_id(), 2);
    }

    #[test]
    fn invalid_exit_reports_no_exit() {
        let mut engine = setup_engine();
        // Room 1 only has a north exit.
        assert_eq!(engine.handle_input("go east"), vec![Event::Message("To the east is no exit".to_string())]);
        assert_eq!(engine.world.current_room_id(), 1);
    }

    #[test]
    fn same_direction_message_from_other_room() {
        let mut engine = setup_engine();
        engine.handle_input("go north"); // now in room 2
        // Room 2 has south and east exits, not west.
        assert_eq!(engine.handle_input("go west"), vec![Event::Message("To the west is no exit".to_string())]);
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
            vec![Event::Took { item: "iron key".to_string() }]
        );
        assert!(engine.world.player_has_item(2));
        assert!(!engine.world.room_has_item(2));
    }

    #[test]
    fn take_unknown_item_reports_not_found() {
        let mut engine = setup_engine();
        assert_eq!(
            engine.handle_input("take bogus"),
            vec![Event::NotFound { phrase: "bogus".to_string() }]
        );
    }

    #[test]
    fn take_ambiguous_name_reports_ambiguous() {
        let mut engine = setup_engine();
        // "key" matches both item 2 (iron key) and item 4 (brass key) in room 1.
        assert_eq!(
            engine.handle_input("take key"),
            vec![Event::Ambiguous { phrase: "key".to_string() }]
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
            vec![Event::Dropped { item: "iron key".to_string() }]
        );
        assert!(!engine.world.player_has_item(2));
        assert!(engine.world.room_has_item(2));
    }

    #[test]
    fn drop_item_not_held_reports_not_found() {
        let mut engine = setup_engine();
        assert_eq!(
            engine.handle_input("drop iron key"),
            vec![Event::NotFound { phrase: "iron key".to_string() }]
        );
    }
}

// --- on_examine ---

mod on_examine {
    use super::*;

    #[test]
    fn examine_found_item_messages() {
        let mut engine = setup_engine();
        assert_eq!(
            messages(&engine.handle_input("examine iron key")),
            vec!["You examine the iron key.".to_string()]
        );
    }

    #[test]
    fn examine_unknown_item_messages() {
        let mut engine = setup_engine();
        assert_eq!(
            messages(&engine.handle_input("examine bogus")),
            vec!["There is no bogus.".to_string()]
        );
    }
}

// --- on_use ---

mod on_use {
    use super::*;

    #[test]
    fn use_unheld_item_reports_missing() {
        let mut engine = setup_engine();
        assert_eq!(
            messages(&engine.handle_input("use sword")),
            vec!["You don't have a sword.".to_string()]
        );
    }

    #[test]
    fn use_held_item_without_target() {
        let mut engine = setup_engine();
        engine.handle_input("take iron key");
        assert_eq!(
            engine.handle_input("use iron key"),
            vec![Event::Used { item: "iron key".to_string(), target: None }]
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
    fn use_held_item_on_unknown_target_reports() {
        let mut engine = setup_engine();
        engine.handle_input("take iron key");
        assert_eq!(
            messages(&engine.handle_input("use iron key on bogus")),
            vec!["You can't use that on bogus.".to_string()]
        );
    }
}

// --- on_unknown ---

mod on_unknown {
    use super::*;

    #[test]
    fn unknown_command_messages() {
        let mut engine = setup_engine();
        assert_eq!(
            messages(&engine.handle_input("dance wildly")),
            vec!["I don't understand how to \"dance wildly\".".to_string()]
        );
    }

    #[test]
    fn unknown_direction_messages() {
        let mut engine = setup_engine();
        assert_eq!(
            messages(&engine.handle_input("go sideways")),
            vec!["I don't understand how to \"go sideways\".".to_string()]
        );
    }
}
