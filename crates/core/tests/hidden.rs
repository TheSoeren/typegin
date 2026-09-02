//! Hidden content: hidden items and hidden exits.
//!
//! These are exposed as *public* helper functions on `WorldState` so a
//! front-end can decide when to reveal them. The engine itself never calls
//! them — a `View` or a custom `Rules` implementation triggers the reveal.
//!
//! The multi-room fixture places hidden content in:
//!   - room 1: hidden item 5 (stale bread)
//!   - room 3: hidden north exit to room 1
//!
//! Run with: cd crates/core && cargo test --test hidden

mod common;

use common::setup_engine;
use core::{Direction, DirectionResolution, Event, ItemId, ItemResolution, RoomId, WorldState};

/// Whether the current room lists `name` among its visible items.
fn room_shows(world: &WorldState, name: &str) -> bool {
    world.room_item_names().iter().any(|n| n == name)
}

/// Whether the current room hides an item under `id` (i.e. it is not among
/// the visible room items).
fn room_hides(world: &WorldState, id: i32) -> bool {
    world.get_item_from_room(ItemId::new(id)) == ItemResolution::NotFound
}

mod hidden_items {
    use super::*;

    #[test]
    fn hidden_item_is_not_listed_among_room_items() {
        let engine = setup_engine();
        assert!(!room_shows(engine.world(), "stale bread"));
        assert!(room_hides(engine.world(), 5));
    }

    #[test]
    fn hidden_item_cannot_be_taken_before_reveal() {
        let mut engine = setup_engine();
        assert_eq!(
            engine.handle_input("take stale bread"),
            vec![Event::TookItemNotFound {
                item: "stale bread".to_string()
            }]
        );
    }

    #[test]
    fn reveal_item_moves_it_to_visible_items() {
        let mut engine = setup_engine();
        assert_eq!(
            engine.world_mut().reveal_item(ItemId::new(5)),
            ItemResolution::Found(ItemId::new(5))
        );
        assert!(room_shows(engine.world(), "stale bread"));
        assert!(!room_hides(engine.world(), 5));
    }

    #[test]
    fn reveal_unknown_item_returns_not_found() {
        let mut engine = setup_engine();
        assert_eq!(
            engine.world_mut().reveal_item(ItemId::new(99)),
            ItemResolution::NotFound
        );
    }

    #[test]
    fn revealed_item_can_be_taken() {
        let mut engine = setup_engine();
        engine.world_mut().reveal_item(ItemId::new(5));
        assert_eq!(
            engine.handle_input("take stale bread"),
            vec![Event::Took {
                item: "stale bread".to_string()
            }]
        );
    }

    #[test]
    fn hide_visible_item_moves_it_to_hidden() {
        let mut engine = setup_engine();
        assert_eq!(
            engine.world_mut().hide_item(ItemId::new(1)),
            ItemResolution::Found(ItemId::new(1))
        );
        assert!(!room_shows(engine.world(), "glowing mysterious sword"));
        assert!(room_hides(engine.world(), 1));
    }

    #[test]
    fn hidden_item_cannot_be_taken_after_hide() {
        let mut engine = setup_engine();
        engine.world_mut().hide_item(ItemId::new(2));
        assert_eq!(
            engine.handle_input("take iron key"),
            vec![Event::TookItemNotFound {
                item: "iron key".to_string()
            }]
        );
    }

    #[test]
    fn hiding_unknown_item_returns_not_found() {
        let mut engine = setup_engine();
        assert_eq!(
            engine.world_mut().hide_item(ItemId::new(99)),
            ItemResolution::NotFound
        );
    }

    #[test]
    fn hide_reveal_round_trip_restores_item() {
        let mut engine = setup_engine();
        engine.world_mut().hide_item(ItemId::new(1));
        assert_eq!(
            engine.world_mut().reveal_item(ItemId::new(1)),
            ItemResolution::Found(ItemId::new(1))
        );
        assert!(room_shows(engine.world(), "glowing mysterious sword"));
        assert!(!room_hides(engine.world(), 1));
    }

    #[test]
    fn reveal_then_hide_round_trip_restores_hidden() {
        let mut engine = setup_engine();
        engine.world_mut().reveal_item(ItemId::new(5));
        assert_eq!(
            engine.world_mut().hide_item(ItemId::new(5)),
            ItemResolution::Found(ItemId::new(5))
        );
        assert!(!room_shows(engine.world(), "stale bread"));
        assert!(room_hides(engine.world(), 5));
    }
}

mod hidden_exits {
    use super::*;

    /// Navigate from room 1 (start) to room 3 (dead end with hidden north exit).
    fn engine_at_room_3() -> (core::GameEngine,) {
        let mut engine = setup_engine();
        assert_eq!(
            engine.handle_input("go north"),
            vec![Event::Went(Direction::North)]
        );
        assert_eq!(
            engine.handle_input("go east"),
            vec![Event::Went(Direction::East)]
        );
        assert_eq!(engine.world().current_room_id(), RoomId::new(3));
        (engine,)
    }

    #[test]
    fn hidden_exit_is_not_traversable_before_reveal() {
        let (mut engine,) = engine_at_room_3();
        assert_eq!(
            engine.handle_input("go north"),
            vec![Event::WentInvalidDirection(Direction::North)]
        );
        assert_eq!(engine.world().current_room_id(), RoomId::new(3));
    }

    #[test]
    fn hidden_exit_direction_is_listed_but_not_visible() {
        let (engine,) = engine_at_room_3();
        assert!(
            engine
                .world()
                .hidden_exit_directions()
                .contains(&Direction::North)
        );
        assert_eq!(
            engine
                .world()
                .get_room_id_by_exit_direction(Direction::North),
            None
        );
    }

    #[test]
    fn reveal_exit_makes_it_traversable() {
        let (mut engine,) = engine_at_room_3();
        assert_eq!(
            engine.world_mut().reveal_exit(Direction::North),
            DirectionResolution::Found(Direction::North)
        );
        assert_eq!(
            engine.handle_input("go north"),
            vec![Event::Went(Direction::North)]
        );
        assert_eq!(engine.world().current_room_id(), RoomId::new(1));
    }

    #[test]
    fn reveal_exit_moves_it_out_of_hidden() {
        let (mut engine,) = engine_at_room_3();
        engine.world_mut().reveal_exit(Direction::North);
        assert!(
            !engine
                .world()
                .hidden_exit_directions()
                .contains(&Direction::North)
        );
        assert_eq!(
            engine
                .world()
                .get_room_id_by_exit_direction(Direction::North),
            Some(RoomId::new(1))
        );
    }

    #[test]
    fn reveal_missing_direction_returns_not_found() {
        let (mut engine,) = engine_at_room_3();
        assert_eq!(
            engine.world_mut().reveal_exit(Direction::North),
            DirectionResolution::Found(Direction::North)
        );
        // A second reveal of the same direction fails: it is no longer hidden.
        assert_eq!(
            engine.world_mut().reveal_exit(Direction::North),
            DirectionResolution::NotFound
        );
    }

    #[test]
    fn hide_visible_exit_moves_it_to_hidden() {
        let mut engine = setup_engine();
        assert_eq!(
            engine.world_mut().hide_exit(Direction::North),
            DirectionResolution::Found(Direction::North)
        );
        assert!(
            engine
                .world()
                .hidden_exit_directions()
                .contains(&Direction::North)
        );
        assert_eq!(
            engine
                .world()
                .get_room_id_by_exit_direction(Direction::North),
            None
        );
    }

    #[test]
    fn hidden_exit_is_not_traversable_after_hide() {
        let mut engine = setup_engine();
        engine.world_mut().hide_exit(Direction::North);
        assert_eq!(
            engine.handle_input("go north"),
            vec![Event::WentInvalidDirection(Direction::North)]
        );
        assert_eq!(engine.world().current_room_id(), RoomId::new(1));
    }

    #[test]
    fn hiding_unknown_direction_returns_not_found() {
        let mut engine = setup_engine();
        assert_eq!(
            engine.world_mut().hide_exit(Direction::East),
            DirectionResolution::NotFound
        );
    }

    #[test]
    fn hide_reveal_round_trip_restores_exit() {
        let mut engine = setup_engine();
        engine.world_mut().hide_exit(Direction::North);
        assert_eq!(
            engine.world_mut().reveal_exit(Direction::North),
            DirectionResolution::Found(Direction::North)
        );
        assert_eq!(
            engine
                .world()
                .get_room_id_by_exit_direction(Direction::North),
            Some(RoomId::new(2))
        );
        assert!(
            !engine
                .world()
                .hidden_exit_directions()
                .contains(&Direction::North)
        );
    }

    #[test]
    fn reveal_then_hide_round_trip_restores_hidden() {
        let (mut engine,) = engine_at_room_3();
        engine.world_mut().reveal_exit(Direction::North);
        assert_eq!(
            engine.world_mut().hide_exit(Direction::North),
            DirectionResolution::Found(Direction::North)
        );
        assert!(
            engine
                .world()
                .hidden_exit_directions()
                .contains(&Direction::North)
        );
        assert!(
            !engine
                .world()
                .get_room_id_by_exit_direction(Direction::North)
                .is_some()
        );
    }
}
