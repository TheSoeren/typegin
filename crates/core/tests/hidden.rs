//! Hidden content: hidden items and hidden exits.
//!
//! These are exposed as *public* helper functions on `WorldState` so a
//! front-end can decide when to reveal them. The engine itself never calls
//! them — a `View` or a custom `Rules` implementation triggers the reveal.
//!
//! The multi-room fixture places hidden content in:
//!   - cellar: hidden item `stale-bread`
//!   - study: hidden north exit back to the cellar
//!
//! Run with: cd crates/core && cargo test --test hidden

mod common;

use common::setup_engine;
use core::{Direction, DirectionResolution, Event, ObjectId, ObjectResolution, RoomId, WorldState};

/// Whether the current room lists `name` among its visible items.
fn room_shows(world: &WorldState, name: &str) -> bool {
    world.room_object_names().iter().any(|n| n == name)
}

/// Whether the current room hides an item under `id` (i.e. it is not among
/// the visible room items).
fn room_hides(world: &WorldState, id: &ObjectId) -> bool {
    world.get_object_from_room(id) == ObjectResolution::NotFound
}

mod hidden_items {
    use super::*;

    #[test]
    fn hidden_item_is_not_listed_among_room_items() {
        let engine = setup_engine();
        assert!(!room_shows(engine.world(), "stale bread"));
        assert!(room_hides(engine.world(), &ObjectId::new("stale-bread")));
    }

    #[test]
    fn hidden_item_cannot_be_taken_before_reveal() {
        let mut engine = setup_engine();
        assert_eq!(
            engine.handle_input("take stale bread"),
            vec![Event::TookObjectNotFound {
                object: "stale bread".to_string()
            }]
        );
    }

    #[test]
    fn reveal_object_moves_it_to_visible_items() {
        let mut engine = setup_engine();
        assert_eq!(
            engine
                .world_mut()
                .reveal_object(&ObjectId::new("stale-bread")),
            ObjectResolution::Found(ObjectId::new("stale-bread"))
        );
        assert!(room_shows(engine.world(), "stale bread"));
        assert!(!room_hides(engine.world(), &ObjectId::new("stale-bread")));
    }

    #[test]
    fn reveal_unknown_item_returns_not_found() {
        let mut engine = setup_engine();
        assert_eq!(
            engine
                .world_mut()
                .reveal_object(&ObjectId::new("nonexistent")),
            ObjectResolution::NotFound
        );
    }

    #[test]
    fn revealed_item_can_be_taken() {
        let mut engine = setup_engine();
        engine
            .world_mut()
            .reveal_object(&ObjectId::new("stale-bread"));
        assert_eq!(
            engine.handle_input("take stale bread"),
            vec![Event::Took {
                object_id: ObjectId::new("stale-bread"),
                object: "stale bread".to_string()
            }]
        );
    }

    #[test]
    fn hide_visible_item_moves_it_to_hidden() {
        let mut engine = setup_engine();
        assert_eq!(
            engine
                .world_mut()
                .hide_object(&ObjectId::new("glowing-sword")),
            ObjectResolution::Found(ObjectId::new("glowing-sword"))
        );
        assert!(!room_shows(engine.world(), "glowing mysterious sword"));
        assert!(room_hides(engine.world(), &ObjectId::new("glowing-sword")));
    }

    #[test]
    fn hidden_item_cannot_be_taken_after_hide() {
        let mut engine = setup_engine();
        engine.world_mut().hide_object(&ObjectId::new("iron-key"));
        assert_eq!(
            engine.handle_input("take iron key"),
            vec![Event::TookObjectNotFound {
                object: "iron key".to_string()
            }]
        );
    }

    #[test]
    fn hiding_unknown_item_returns_not_found() {
        let mut engine = setup_engine();
        assert_eq!(
            engine
                .world_mut()
                .hide_object(&ObjectId::new("nonexistent")),
            ObjectResolution::NotFound
        );
    }

    #[test]
    fn hide_reveal_round_trip_restores_item() {
        let mut engine = setup_engine();
        engine
            .world_mut()
            .hide_object(&ObjectId::new("glowing-sword"));
        assert_eq!(
            engine
                .world_mut()
                .reveal_object(&ObjectId::new("glowing-sword")),
            ObjectResolution::Found(ObjectId::new("glowing-sword"))
        );
        assert!(room_shows(engine.world(), "glowing mysterious sword"));
        assert!(!room_hides(engine.world(), &ObjectId::new("glowing-sword")));
    }

    #[test]
    fn reveal_then_hide_round_trip_restores_hidden() {
        let mut engine = setup_engine();
        engine
            .world_mut()
            .reveal_object(&ObjectId::new("stale-bread"));
        assert_eq!(
            engine
                .world_mut()
                .hide_object(&ObjectId::new("stale-bread")),
            ObjectResolution::Found(ObjectId::new("stale-bread"))
        );
        assert!(!room_shows(engine.world(), "stale bread"));
        assert!(room_hides(engine.world(), &ObjectId::new("stale-bread")));
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
        assert_eq!(engine.world().current_room_id(), RoomId::new("study"));
        (engine,)
    }

    #[test]
    fn hidden_exit_is_not_traversable_before_reveal() {
        let (mut engine,) = engine_at_room_3();
        assert_eq!(
            engine.handle_input("go north"),
            vec![Event::WentExitHidden(Direction::North)]
        );
        assert_eq!(engine.world().current_room_id(), RoomId::new("study"));
    }

    #[test]
    fn hidden_exit_direction_is_listed_but_not_visible() {
        let (engine,) = engine_at_room_3();
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
        assert_eq!(engine.world().current_room_id(), RoomId::new("cellar"));
    }

    #[test]
    fn reveal_exit_moves_it_out_of_hidden() {
        let (mut engine,) = engine_at_room_3();
        engine.world_mut().reveal_exit(Direction::North);
        assert_eq!(
            engine
                .world()
                .get_room_id_by_exit_direction(Direction::North),
            Some(RoomId::new("cellar"))
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
            vec![Event::WentExitHidden(Direction::North)]
        );
        assert_eq!(engine.world().current_room_id(), RoomId::new("cellar"));
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
            Some(RoomId::new("corridor"))
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
        assert_eq!(
            engine
                .world()
                .get_room_id_by_exit_direction(Direction::North),
            None
        );
    }
}
