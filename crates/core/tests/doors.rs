//! Spec (red phase) for door state.
//!
//! A door is a Scene object in the room's object list carrying optional door
//! data: a destination plus a `locked` flag. An exit's *hidden* flag no longer
//! lives on the door — hidden-ness is list membership: a hidden door is an
//! object sitting in the room's `hidden_objects` until revealed. Both locked
//! and hidden refuse movement; the engine reports the reason
//! (`WentExitHidden` vs `WentExitLocked`) so the consumer decides how a
//! hidden exit reads (undiscovered passage vs dead end). World data declares
//! door objects once in the object table; each room just lists which of them
//! it holds.

mod common;

use core::Direction;
use core::DirectionResolution;
use core::{Event, RoomId};

/// Navigate from room 1 (start) to room 3, whose doors are:
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
    fn door_object_parses_destination() {
        let data = common::multi_room_world_data();
        let stairs = data.find_object(8).expect("door object 8 exists");
        let door = stairs.door.as_ref().expect("door data");
        assert_eq!(door.to, 2);
        assert_eq!(door.direction, "north".to_string());
        assert!(!door.locked);
        assert_eq!(stairs.kind, core::ObjectKind::Scene);
    }

    #[test]
    fn door_object_parses_locked_and_gated_flags() {
        let data = common::multi_room_world_data();
        let oak = data.find_object(14).expect("oak door");
        let oak_door = oak.door.as_ref().expect("door data");
        assert_eq!(oak_door.to, 2);
        assert!(oak_door.locked);
        assert_eq!(oak_door.gated_by, Some(2));

        let vault = data.find_object(13).expect("hidden vault");
        assert!(vault.door.as_ref().expect("door data").locked);
    }

    #[test]
    fn hidden_door_is_listed_as_hidden_in_its_room() {
        let data = common::multi_room_world_data();
        let room = data.find_room(3).expect("room 3 exists");
        assert!(room.hidden_objects.contains(&12)); // secret passage
        assert!(room.hidden_objects.contains(&13)); // hidden vault
        assert!(room.visible_objects.contains(&11)); // wooden door is visible
    }

    #[test]
    fn door_without_flags_defaults_to_open() {
        let data = common::multi_room_world_data();
        for id in [8, 9, 10, 11] {
            let object = data.find_object(id).expect("door object exists");
            let door = object.door.as_ref().expect("door data");
            assert!(!door.locked, "door {id} must default to unlocked");
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
