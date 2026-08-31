//! Drop action: lexer, rules hook, world-state movement, engine, and the
//! combined navigation + drop flows.
//!
//! Run with: cd crates/core && cargo test --test drop

mod common;

use core::{Action, BasicRules, Event, parse_input};
use common::{setup_engine, setup_engine_with_rules};

mod drop_lexer {
    use super::*;

    #[test]
    fn parse_drop_item() {
        let action = parse_input("drop sword");
        assert_eq!(action, Action::Drop("sword".to_string()));
    }

    #[test]
    fn parse_drop_with_descriptor() {
        let action = parse_input("drop the iron key");
        assert_eq!(action, Action::Drop("iron key".to_string()));
    }

    #[test]
    fn parse_drop_shortcut() {
        let action = parse_input("d sword");
        assert_eq!(action, Action::Drop("sword".to_string()));
    }

    #[test]
    fn parse_drop_empty_is_unknown() {
        let action = parse_input("drop");
        assert_eq!(action, Action::Unknown("drop".to_string()));
    }
}

mod rules_drop_trait {
    use super::*;

    #[test]
    fn default_rules_drop_returns_dropped_event() {
        let mut engine = setup_engine_with_rules(BasicRules);
        engine.handle_input("take iron key");
        let events = engine.handle_input("drop iron key");
        assert_eq!(
            events,
            vec![Event::Dropped {
                item: "iron key".to_string()
            }]
        );
    }
}

mod world_state_drop {
    use super::*;

    #[test]
    fn move_item_from_inventory_to_room() {
        let mut engine = setup_engine();
        engine.world.move_item_to_inventory(2); // take iron key
        assert!(engine.world.player_has_item(2));

        let moved = engine.world.move_item_from_inventory(2);
        assert!(moved);
        assert!(!engine.world.player_has_item(2));
        assert!(engine.world.room_has_item(2));
    }

    #[test]
    fn drop_item_not_in_inventory_fails() {
        let mut engine = setup_engine();
        let moved = engine.world.move_item_from_inventory(99);
        assert!(!moved);
    }

    #[test]
    fn drop_item_not_held_returns_false() {
        let mut engine = setup_engine();
        let moved = engine.world.move_item_from_inventory(2); // not holding it
        assert!(!moved);
        assert!(engine.world.room_has_item(2));
    }
}

mod engine_drop {
    use super::*;

    #[test]
    fn drop_item_moves_it_to_room() {
        let mut engine = setup_engine();
        engine.handle_input("take iron key");
        assert!(engine.world.player_has_item(2));

        let events = engine.handle_input("drop iron key");
        assert_eq!(
            events,
            vec![Event::Dropped {
                item: "iron key".to_string()
            }]
        );
        assert!(!engine.world.player_has_item(2));
        assert!(engine.world.room_has_item(2));
    }

    #[test]
    fn drop_item_not_in_inventory() {
        let mut engine = setup_engine();
        let events = engine.handle_input("drop iron key");
        assert_eq!(
            events,
            vec![Event::NotFound {
                phrase: "iron key".to_string()
            }]
        );
    }

    #[test]
    fn drop_ambiguous_item() {
        let mut engine = setup_engine();
        engine.handle_input("take iron key");
        engine.handle_input("take brass key");
        let events = engine.handle_input("drop key");
        assert_eq!(
            events,
            vec![Event::Ambiguous {
                phrase: "key".to_string()
            }]
        );
    }

    #[test]
    fn drop_nonexistent_item() {
        let mut engine = setup_engine();
        let events = engine.handle_input("drop ghost armor");
        assert_eq!(
            events,
            vec![Event::NotFound {
                phrase: "ghost armor".to_string()
            }]
        );
    }

    #[test]
    fn drop_item_then_take_again() {
        let mut engine = setup_engine();
        engine.handle_input("take iron key");
        assert!(engine.world.player_has_item(2));

        engine.handle_input("drop iron key");
        assert!(!engine.world.player_has_item(2));
        assert!(engine.world.room_has_item(2));

        engine.handle_input("take iron key");
        assert!(engine.world.player_has_item(2));
        assert!(!engine.world.room_has_item(2));
    }
}

mod integration {
    use super::*;

    #[test]
    fn take_item_in_room_1_move_to_room_2_drop_there() {
        let mut engine = setup_engine();

        engine.handle_input("take sword");
        assert!(engine.world.player_has_item(1));

        engine.handle_input("n");
        assert_eq!(engine.world.current_room_id(), 2);

        engine.handle_input("drop sword");
        assert!(!engine.world.player_has_item(1));
        assert!(engine.world.room_has_item(1));

        engine.handle_input("s");
        assert_eq!(engine.world.current_room_id(), 1);
        assert!(!engine.world.room_has_item(1));
    }

    #[test]
    fn full_adventure_flow() {
        let mut engine = setup_engine();

        let events = engine.handle_input("look");
        assert_eq!(events, vec![Event::Looked]);
        assert!(
            engine
                .world
                .room_item_names()
                .contains(&"glowing mysterious sword".to_string())
        );

        engine.handle_input("take iron key");
        engine.handle_input("take brass key");
        assert_eq!(engine.world.inventory_item_names().len(), 2);

        engine.handle_input("go north");
        assert_eq!(engine.world.current_room_id(), 2);

        let events = engine.handle_input("look");
        assert_eq!(events, vec![Event::Looked]);
        assert!(
            engine
                .world
                .room_item_names()
                .contains(&"rusty lamp".to_string())
        );

        engine.handle_input("take lamp");
        assert!(engine.world.player_has_item(6));

        engine.handle_input("go east");
        assert_eq!(engine.world.current_room_id(), 3);

        let events = engine.handle_input("go north");
        assert_eq!(
            events,
            vec![Event::Message("To the north is no exit".to_string())]
        );

        engine.handle_input("go west");
        assert_eq!(engine.world.current_room_id(), 2);

        engine.handle_input("drop lamp");
        assert!(!engine.world.player_has_item(6));
        assert!(engine.world.room_has_item(6));

        engine.handle_input("go south");
        assert_eq!(engine.world.current_room_id(), 1);

        assert!(engine.world.player_has_item(2));
        assert!(engine.world.player_has_item(4));
    }
}
