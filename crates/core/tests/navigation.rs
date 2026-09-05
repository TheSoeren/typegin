//! Room navigation: exits data, world-state movement, and the engine `on_go`.
//!
//! Run with: cd crates/core && cargo test --test navigation

mod common;

use common::{multi_room_world_data, setup_engine};
use core::{Direction, Event, RoomId};

mod room_exits {
    use super::*;

    #[test]
    fn room_data_has_exits() {
        let data = multi_room_world_data();
        let room1 = data.rooms.iter().find(|r| r.id == 1).unwrap();
        assert_eq!(room1.exits.get("north").map(|e| e.to), Some(2));
    }

    #[test]
    fn room_data_multiple_exits() {
        let data = multi_room_world_data();
        let room2 = data.rooms.iter().find(|r| r.id == 2).unwrap();
        assert_eq!(room2.exits.get("south").map(|e| e.to), Some(1));
        assert_eq!(room2.exits.get("east").map(|e| e.to), Some(3));
    }

    #[test]
    fn room_data_dead_end() {
        let data = multi_room_world_data();
        let room3 = data.rooms.iter().find(|r| r.id == 3).unwrap();
        let open_exits = room3
            .exits
            .values()
            .filter(|e| !e.hidden && !e.locked)
            .count();
        assert_eq!(open_exits, 1);
    }
}

mod world_state_navigation {
    use super::*;

    #[test]
    fn world_tracks_current_room_id() {
        let engine = setup_engine();
        assert_eq!(engine.world().current_room_id(), RoomId::new(1));
    }

    #[test]
    fn world_can_change_room() {
        let mut engine = setup_engine();
        engine.world_mut().move_to_room(RoomId::new(2));
        assert_eq!(engine.world().current_room_id(), RoomId::new(2));
    }

    #[test]
    fn room_items_change_after_moving() {
        let mut engine = setup_engine();
        assert!(
            engine
                .world()
                .room_item_names()
                .contains(&"glowing mysterious sword".to_string())
        );

        engine.world_mut().move_to_room(RoomId::new(2));
        assert!(
            engine
                .world()
                .room_item_names()
                .contains(&"rusty lamp".to_string())
        );
        assert!(
            !engine
                .world()
                .room_item_names()
                .contains(&"glowing mysterious sword".to_string())
        );
    }

    #[test]
    fn invalid_move_does_not_change_room() {
        let engine = setup_engine();
        let target = engine
            .world()
            .get_room_id_by_exit_direction(Direction::West);
        assert_eq!(target, None);
        assert_eq!(engine.world().current_room_id(), RoomId::new(1));
    }

    #[test]
    fn valid_move_returns_target_room_id() {
        let engine = setup_engine();
        let target = engine
            .world()
            .get_room_id_by_exit_direction(Direction::North);
        assert_eq!(target, Some(RoomId::new(2)));
    }

    #[test]
    fn move_from_dead_end_fails() {
        let mut engine = setup_engine();
        engine.world_mut().move_to_room(RoomId::new(3)); // dead end
        let target = engine
            .world()
            .get_room_id_by_exit_direction(Direction::North);
        assert_eq!(target, None);
        assert_eq!(engine.world().current_room_id(), RoomId::new(3));
    }

    #[test]
    fn exit_directions_lists_passable_doors() {
        let engine = setup_engine();
        assert_eq!(engine.world().exit_directions(), vec![Direction::North]);
    }

    #[test]
    fn locked_and_hidden_exits_are_not_listed_as_open() {
        let mut engine = setup_engine();
        engine.handle_input("go north");
        engine.handle_input("go east"); // now in room 3
        assert_eq!(engine.world().exit_directions(), vec![Direction::West]);
    }
}

mod engine_navigation {
    use super::*;

    #[test]
    fn go_north_moves_to_room_2() {
        let mut engine = setup_engine();
        let events = engine.handle_input("go north");
        assert_eq!(events, vec![Event::Went(Direction::North)]);
        assert_eq!(engine.world().current_room_id(), RoomId::new(2));
    }

    #[test]
    fn go_south_from_room_2_returns_to_room_1() {
        let mut engine = setup_engine();
        engine.handle_input("go north");
        let events = engine.handle_input("go south");
        assert_eq!(events, vec![Event::Went(Direction::South)]);
        assert_eq!(engine.world().current_room_id(), RoomId::new(1));
    }

    #[test]
    fn go_to_dead_end_then_back() {
        let mut engine = setup_engine();
        engine.handle_input("go north");
        engine.handle_input("go east");
        assert_eq!(engine.world().current_room_id(), RoomId::new(3));

        let events = engine.handle_input("go west");
        assert_eq!(events, vec![Event::Went(Direction::West)]);
        assert_eq!(engine.world().current_room_id(), RoomId::new(2));
    }

    #[test]
    fn go_invalid_direction_stays_in_room() {
        let mut engine = setup_engine();
        let events = engine.handle_input("go west");
        assert_eq!(events, vec![Event::WentInvalidDirection(Direction::West)]);
        assert_eq!(engine.world().current_room_id(), RoomId::new(1));
    }

    #[test]
    fn go_north_shortcut() {
        let mut engine = setup_engine();
        let events = engine.handle_input("n");
        assert_eq!(events, vec![Event::Went(Direction::North)]);
        assert_eq!(engine.world().current_room_id(), RoomId::new(2));
    }

    #[test]
    fn look_after_moving_shows_new_room() {
        let mut engine = setup_engine();
        engine.handle_input("go north");
        let events = engine.handle_input("look");
        assert_eq!(events, vec![Event::Looked]);
        assert!(
            engine
                .world()
                .room_item_names()
                .contains(&"rusty lamp".to_string())
        );
    }
}
