//! Spec (red phase) for exit state.
//!
//! An exit is a value — it has a destination plus two independent flags:
//! `hidden` and `locked`. An exit can be both (a secret door that also needs
//! a key). Both refuse movement; the engine reports the reason
//! (`WentExitHidden` vs `WentExitLocked`) so the consumer decides how a
//! hidden exit reads (undiscovered passage vs dead end). World data declares
//! one `exits` table per room, so exit state lives in a single map on the
//! room instead of parallel per-flag maps.

mod common;

use core::Direction;
use core::DirectionResolution;
use core::{Event, RoomId};

/// Navigate from room 1 (start) to room 3, whose exits are:
/// west (open), north (hidden), east (locked), south (hidden + locked).
fn engine_at_room_3() -> core::GameEngine {
    let mut engine = common::setup_engine();
    assert_eq!(
        engine.handle_input("go north"),
        vec![Event::Went(Direction::North)]
    );
    assert_eq!(
        engine.handle_input("go east"),
        vec![Event::Went(Direction::East)]
    );
    assert_eq!(engine.world().current_room_id(), RoomId::new(3));
    engine
}

// --- TOML data parsing ---

mod data_parsing {
    use super::*;

    #[test]
    fn room_data_parses_exit_destinations() {
        let data = common::multi_room_world_data();
        let room = data.find_room(1).expect("room 1 exists");
        let north = room.exits.get("north").expect("north exit");
        assert_eq!(north.to, 2);
        assert!(!north.locked);
        assert!(!north.hidden);
    }

    #[test]
    fn room_data_parses_locked_and_hidden_flags() {
        let data = common::multi_room_world_data();
        let room = data.find_room(3).expect("room 3 exists");
        let east = room.exits.get("east").expect("east exit");
        assert_eq!(east.to, 2);
        assert!(east.locked);
        assert!(!east.hidden);

        let north = room.exits.get("north").expect("north exit");
        assert_eq!(north.to, 1);
        assert!(!north.locked);
        assert!(north.hidden);

        let south = room.exits.get("south").expect("south exit");
        assert_eq!(south.to, 1);
        assert!(south.locked);
        assert!(south.hidden);
    }

    #[test]
    fn exit_without_flags_defaults_to_open() {
        let data = common::multi_room_world_data();
        let room = data.find_room(2).expect("room 2 exists");
        for exit in room.exits.values() {
            assert!(!exit.locked);
            assert!(!exit.hidden);
        }
    }
}

// --- World state door handling ---

mod world_state_doors {
    use super::*;

    #[test]
    fn locked_exit_is_not_traversable_before_unlock() {
        let mut engine = engine_at_room_3();
        assert_eq!(
            engine.handle_input("go east"),
            vec![Event::WentExitLocked(Direction::East)]
        );
        assert_eq!(engine.world().current_room_id(), RoomId::new(3));
    }

    #[test]
    fn locked_direction_is_listed_but_not_an_exit() {
        let engine = engine_at_room_3();
        assert!(engine.world().is_exit_locked(Direction::East));
        assert_eq!(
            engine
                .world()
                .get_room_id_by_exit_direction(Direction::East),
            None
        );
    }

    #[test]
    fn unlock_exit_makes_it_traversable() {
        let mut engine = engine_at_room_3();
        assert_eq!(
            engine.world_mut().unlock_exit(Direction::East),
            DirectionResolution::Found(Direction::East)
        );
        assert!(!engine.world().is_exit_locked(Direction::East));
        assert_eq!(
            engine.handle_input("go east"),
            vec![Event::Went(Direction::East)]
        );
        assert_eq!(engine.world().current_room_id(), RoomId::new(2));
    }

    #[test]
    fn unlock_absent_direction_returns_not_found() {
        let mut engine = common::setup_engine();
        assert_eq!(
            engine.world_mut().unlock_exit(Direction::East),
            DirectionResolution::NotFound
        );
    }

    #[test]
    fn lock_visible_exit_blocks_it() {
        let mut engine = common::setup_engine();
        assert_eq!(
            engine.world_mut().lock_exit(Direction::North),
            DirectionResolution::Found(Direction::North)
        );
        assert!(engine.world().is_exit_locked(Direction::North));
        assert_eq!(
            engine.handle_input("go north"),
            vec![Event::WentExitLocked(Direction::North)]
        );
        assert_eq!(engine.world().current_room_id(), RoomId::new(1));
    }

    #[test]
    fn lock_unknown_direction_returns_not_found() {
        let mut engine = common::setup_engine();
        assert_eq!(
            engine.world_mut().lock_exit(Direction::East),
            DirectionResolution::NotFound
        );
    }

    #[test]
    fn lock_unlock_round_trip_restores_exit() {
        let mut engine = common::setup_engine();
        engine.world_mut().lock_exit(Direction::North);
        assert_eq!(
            engine.world_mut().unlock_exit(Direction::North),
            DirectionResolution::Found(Direction::North)
        );
        assert!(!engine.world().is_exit_locked(Direction::North));
        assert_eq!(
            engine.handle_input("go north"),
            vec![Event::Went(Direction::North)]
        );
    }

    #[test]
    fn unlock_lock_round_trip_restores_locked() {
        let mut engine = engine_at_room_3();
        engine.world_mut().unlock_exit(Direction::East);
        assert_eq!(
            engine.world_mut().lock_exit(Direction::East),
            DirectionResolution::Found(Direction::East)
        );
        assert_eq!(
            engine.handle_input("go east"),
            vec![Event::WentExitLocked(Direction::East)]
        );
    }

    #[test]
    fn hidden_exit_stays_unlisted_and_untraversable() {
        let mut engine = engine_at_room_3();
        assert!(engine.world().is_exit_hidden(Direction::North));
        assert_eq!(
            engine.handle_input("go north"),
            vec![Event::WentExitHidden(Direction::North)]
        );
        assert_eq!(engine.world().current_room_id(), RoomId::new(3));
    }

    #[test]
    fn hidden_and_locked_exit_hides_its_lock_status() {
        let mut engine = engine_at_room_3();
        assert!(engine.world().is_exit_hidden(Direction::South));
        assert!(engine.world().is_exit_locked(Direction::South));
        assert_eq!(
            engine.handle_input("go south"),
            vec![Event::WentExitHidden(Direction::South)]
        );
        assert_eq!(engine.world().current_room_id(), RoomId::new(3));
    }

    #[test]
    fn revealing_hidden_and_locked_exit_exposes_the_door() {
        let mut engine = engine_at_room_3();
        assert_eq!(
            engine.world_mut().reveal_exit(Direction::South),
            DirectionResolution::Found(Direction::South)
        );
        assert!(!engine.world().is_exit_hidden(Direction::South));
        assert!(engine.world().is_exit_locked(Direction::South));
        assert_eq!(
            engine.handle_input("go south"),
            vec![Event::WentExitLocked(Direction::South)]
        );
        assert_eq!(engine.world().current_room_id(), RoomId::new(3));
    }

    #[test]
    fn non_locked_invalid_direction_still_reports_invalid() {
        let mut engine = common::setup_engine();
        assert_eq!(
            engine.handle_input("go east"),
            vec![Event::WentInvalidDirection(Direction::East)]
        );
    }
}
