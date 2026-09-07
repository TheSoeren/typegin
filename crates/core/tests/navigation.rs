//! Room navigation: exits data, world-state movement, and the engine `on_go`.
//!
//! Run with: cd crates/core && cargo test --test navigation

mod common;

use common::{multi_room_world_data, setup_engine};
use core::{Direction, Event, ObjectId, RoomId};

mod room_door_data {
    use super::*;

    #[test]
    fn door_object_leads_where_the_exit_used_to() {
        let data = multi_room_world_data();
        let stairs = data
            .find_object(&ObjectId::new("cellar-stairs"))
            .expect("cellar stairs object exists");
        assert_eq!(
            stairs.door.as_ref().expect("door data").to,
            "corridor".to_string()
        );
        assert_eq!(
            stairs.door.as_ref().expect("door data").direction,
            "north".to_string()
        );
    }

    #[test]
    fn multiple_doors_serve_one_room() {
        let data = multi_room_world_data();
        let corridor = data
            .find_room(&RoomId::new("corridor"))
            .expect("corridor exists");
        assert!(
            corridor
                .visible_objects
                .contains(&ObjectId::new("corridor-stairs"))
        );
        assert!(
            corridor
                .visible_objects
                .contains(&ObjectId::new("study-door"))
        );
    }

    #[test]
    fn dead_end_room_holds_one_open_door() {
        let data = multi_room_world_data();
        let study = data.find_room(&RoomId::new("study")).expect("study exists");
        let open = study
            .visible_objects
            .iter()
            .filter(|id| {
                let object = data.find_object(id).expect("object in room");
                object.door.as_ref().is_some_and(|door| !door.locked)
            })
            .count();
        assert_eq!(open, 1);
    }
}

mod world_state_navigation {
    use super::*;

    #[test]
    fn world_tracks_current_room_id() {
        let engine = setup_engine();
        assert_eq!(engine.world().current_room_id(), RoomId::new("cellar"));
    }

    #[test]
    fn world_can_change_room() {
        let mut engine = setup_engine();
        engine.world_mut().move_to_room(RoomId::new("corridor"));
        assert_eq!(engine.world().current_room_id(), RoomId::new("corridor"));
    }

    #[test]
    fn room_items_change_after_moving() {
        let mut engine = setup_engine();
        assert!(
            engine
                .world()
                .room_object_names()
                .contains(&"glowing mysterious sword".to_string())
        );

        engine.world_mut().move_to_room(RoomId::new("corridor"));
        assert!(
            engine
                .world()
                .room_object_names()
                .contains(&"rusty lamp".to_string())
        );
        assert!(
            !engine
                .world()
                .room_object_names()
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
        assert_eq!(engine.world().current_room_id(), RoomId::new("cellar"));
    }

    #[test]
    fn valid_move_returns_target_room_id() {
        let engine = setup_engine();
        let target = engine
            .world()
            .get_room_id_by_exit_direction(Direction::North);
        assert_eq!(target, Some(RoomId::new("corridor")));
    }

    #[test]
    fn move_from_dead_end_fails() {
        let mut engine = setup_engine();
        engine.world_mut().move_to_room(RoomId::new("study")); // dead end
        let target = engine
            .world()
            .get_room_id_by_exit_direction(Direction::North);
        assert_eq!(target, None);
        assert_eq!(engine.world().current_room_id(), RoomId::new("study"));
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
        engine.handle_input("go east"); // now in study
        assert_eq!(engine.world().exit_directions(), vec![Direction::West]);
    }
}

mod engine_navigation {
    use super::*;

    #[test]
    fn go_north_moves_to_corridor() {
        let mut engine = setup_engine();
        let events = engine.handle_input("go north");
        assert_eq!(events, vec![Event::Went(Direction::North)]);
        assert_eq!(engine.world().current_room_id(), RoomId::new("corridor"));
    }

    #[test]
    fn go_south_from_corridor_returns_to_cellar() {
        let mut engine = setup_engine();
        engine.handle_input("go north");
        let events = engine.handle_input("go south");
        assert_eq!(events, vec![Event::Went(Direction::South)]);
        assert_eq!(engine.world().current_room_id(), RoomId::new("cellar"));
    }

    #[test]
    fn go_to_dead_end_then_back() {
        let mut engine = setup_engine();
        engine.handle_input("go north");
        engine.handle_input("go east");
        assert_eq!(engine.world().current_room_id(), RoomId::new("study"));

        let events = engine.handle_input("go west");
        assert_eq!(events, vec![Event::Went(Direction::West)]);
        assert_eq!(engine.world().current_room_id(), RoomId::new("corridor"));
    }

    #[test]
    fn go_invalid_direction_stays_in_room() {
        let mut engine = setup_engine();
        let events = engine.handle_input("go west");
        assert_eq!(events, vec![Event::WentInvalidDirection(Direction::West)]);
        assert_eq!(engine.world().current_room_id(), RoomId::new("cellar"));
    }

    #[test]
    fn go_north_shortcut() {
        let mut engine = setup_engine();
        let events = engine.handle_input("n");
        assert_eq!(events, vec![Event::Went(Direction::North)]);
        assert_eq!(engine.world().current_room_id(), RoomId::new("corridor"));
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
                .room_object_names()
                .contains(&"rusty lamp".to_string())
        );
    }
}
