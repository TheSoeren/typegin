//! Room navigation: exits data, world-state movement, and the engine `on_go`.
//!
//! Run with: cd crates/core && cargo test --test navigation

mod common;

use common::{multi_room_world_data, setup_engine};
use core::{Direction, Event, GoTarget};

mod room_door_data {
    use super::*;

    #[test]
    fn door_object_leads_where_the_exit_used_to() {
        let data = multi_room_world_data();
        let stairs = data
            .find_object(&core::objectId!("cellar-stairs"))
            .expect("cellar stairs object exists");
        assert_eq!(
            stairs.door.as_ref().expect("door data").to,
            "corridor".to_string()
        );
        assert_eq!(
            stairs.door.as_ref().expect("door data").direction,
            Some("north".to_string())
        );
    }

    #[test]
    fn multiple_doors_serve_one_room() {
        let data = multi_room_world_data();
        let corridor = data
            .find_room(&core::roomId!("corridor"))
            .expect("corridor exists");
        assert!(
            corridor
                .visible_objects
                .contains(&core::objectId!("corridor-stairs"))
        );
        assert!(
            corridor
                .visible_objects
                .contains(&core::objectId!("study-door"))
        );
    }

    #[test]
    fn dead_end_room_holds_one_open_door() {
        let data = multi_room_world_data();
        let study = data
            .find_room(&core::roomId!("study"))
            .expect("study exists");
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
        assert_eq!(engine.world().current_room_id(), core::roomId!("cellar"));
    }

    #[test]
    fn world_can_change_room() {
        let mut engine = setup_engine();
        engine.world_mut().move_to_room(core::roomId!("corridor"));
        assert_eq!(engine.world().current_room_id(), core::roomId!("corridor"));
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

        engine.world_mut().move_to_room(core::roomId!("corridor"));
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
            .get_room_id_by_go_target(&GoTarget::Direction(Direction::West));
        assert_eq!(target, None);
        assert_eq!(engine.world().current_room_id(), core::roomId!("cellar"));
    }

    #[test]
    fn valid_move_returns_target_room_id() {
        let engine = setup_engine();
        let target = engine
            .world()
            .get_room_id_by_go_target(&GoTarget::Direction(Direction::North));
        assert_eq!(target, Some(core::roomId!("corridor")));
    }

    #[test]
    fn move_from_dead_end_fails() {
        let mut engine = setup_engine();
        engine.world_mut().move_to_room(core::roomId!("study")); // dead end
        let target = engine
            .world()
            .get_room_id_by_go_target(&GoTarget::Direction(Direction::North));
        assert_eq!(target, None);
        assert_eq!(engine.world().current_room_id(), core::roomId!("study"));
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

// ---------------------------------------------------------------------------
// Named exits: `Action::Go(GoTarget::Named(..))` reaches a door by name
// instead of by compass direction — the point-and-click-friendly path, and
// the only way to reach a door that has no `direction` at all. A standalone
// world, not the shared `multi_room_world_data()` fixture: every door there
// already has a direction, and forking a per-scenario copy of the shared
// fixture is exactly what AGENTS.md's testing conventions ask suites not to
// do, so a schema shape the shared fixture doesn't cover gets its own
// minimal inline `WorldData` instead (mirrors `symbolic_keys.rs`'s
// `mod integrity` helper for the same reason).
// ---------------------------------------------------------------------------

mod named_exits {
    use super::*;
    use core::{Action, GameEngine, GoTarget, Target, TargetResolution, WorldData};

    /// `hatch` and `gate` both lead from `start` to `garden` with no
    /// `direction` at all; `gate` is locked. Both alias to "door" so a
    /// name lookup can be ambiguous. `rock` is a non-door scene object and
    /// `gardener` an NPC, to prove "resolved fine but isn't a door" is
    /// reported distinctly from "no such object" for both kinds of target.
    fn named_exit_world() -> WorldData {
        WorldData::from_yaml(
            r#"
objects:
  - key: hatch
    primary_name: wooden hatch
    aliases: [door]
    kind: Scene
    door:
      to: garden

  - key: gate
    primary_name: iron gate
    aliases: [door]
    kind: Scene
    door:
      to: garden
      locked: true

  - key: rock
    primary_name: rock
    kind: Scene

rooms:
  - key: start
    visible_objects: [hatch, gate, rock]
  - key: garden
    visible_objects: []

npcs:
  - key: gardener
    primary_name: gardener
    room: start
    dialogue:
      root: greet
      nodes:
        greet:
          text: "Hello."
"#,
        )
        .expect("named-exit world parses")
    }

    #[test]
    fn entering_a_direction_less_door_by_name_moves_the_player() {
        let mut engine = GameEngine::get(&named_exit_world());
        let events = engine.execute_action(Action::Go(GoTarget::Named("wooden hatch".to_string())));
        assert_eq!(
            events,
            vec![Event::Entered {
                object_id: core::objectId!("hatch"),
                object: "wooden hatch".to_string(),
            }]
        );
        assert_eq!(engine.world().current_room_id(), core::roomId!("garden"));
    }

    #[test]
    fn entering_a_locked_direction_less_door_reports_locked_without_moving() {
        let mut engine = GameEngine::get(&named_exit_world());
        let events = engine.execute_action(Action::Go(GoTarget::Named("iron gate".to_string())));
        assert_eq!(
            events,
            vec![Event::EnteredExitLocked {
                object_id: core::objectId!("gate"),
                object: "iron gate".to_string(),
            }]
        );
        assert_eq!(engine.world().current_room_id(), core::roomId!("start"));
    }

    #[test]
    fn entering_a_non_door_object_reports_cant_enter() {
        let mut engine = GameEngine::get(&named_exit_world());
        let events = engine.execute_action(Action::Go(GoTarget::Named("rock".to_string())));
        assert_eq!(
            events,
            vec![Event::CantEnter {
                target: "rock".to_string(),
            }]
        );
        assert_eq!(engine.world().current_room_id(), core::roomId!("start"));
    }

    #[test]
    fn entering_an_npc_reports_cant_enter() {
        let mut engine = GameEngine::get(&named_exit_world());
        let events = engine.execute_action(Action::Go(GoTarget::Named("gardener".to_string())));
        assert_eq!(
            events,
            vec![Event::CantEnter {
                target: "gardener".to_string(),
            }]
        );
        assert_eq!(engine.world().current_room_id(), core::roomId!("start"));
        // Sanity: `gardener` really did resolve to an NPC, not "not found".
        assert_eq!(
            engine.world().resolve_target("gardener"),
            TargetResolution::Found(Target::Npc(core::npcId!("gardener")))
        );
    }

    #[test]
    fn entering_an_unknown_name_reports_not_found() {
        let mut engine = GameEngine::get(&named_exit_world());
        let events = engine.execute_action(Action::Go(GoTarget::Named("nonexistent".to_string())));
        assert_eq!(
            events,
            vec![Event::EnteredTargetNotFound {
                target: "nonexistent".to_string(),
            }]
        );
    }

    #[test]
    fn entering_an_ambiguous_alias_reports_ambiguous() {
        let mut engine = GameEngine::get(&named_exit_world());
        let events = engine.execute_action(Action::Go(GoTarget::Named("door".to_string())));
        assert_eq!(
            events,
            vec![Event::EnteredTargetAmbiguous {
                target_ids: vec![
                    Target::Object(core::objectId!("hatch")),
                    Target::Object(core::objectId!("gate")),
                ],
                target: "door".to_string(),
            }]
        );
    }

    #[test]
    fn the_parser_reaches_a_direction_less_door_via_enter() {
        let mut engine = GameEngine::get(&named_exit_world());
        let events = engine.handle_input("enter wooden hatch");
        assert_eq!(
            events,
            vec![Event::Entered {
                object_id: core::objectId!("hatch"),
                object: "wooden hatch".to_string(),
            }]
        );
        assert_eq!(engine.world().current_room_id(), core::roomId!("garden"));
    }
}

mod engine_navigation {
    use super::*;

    #[test]
    fn go_north_moves_to_corridor() {
        let mut engine = setup_engine();
        let events = engine.handle_input("go north");
        assert_eq!(
            events,
            vec![Event::Went(GoTarget::Direction(Direction::North))]
        );
        assert_eq!(engine.world().current_room_id(), core::roomId!("corridor"));
    }

    #[test]
    fn go_south_from_corridor_returns_to_cellar() {
        let mut engine = setup_engine();
        engine.handle_input("go north");
        let events = engine.handle_input("go south");
        assert_eq!(
            events,
            vec![Event::Went(GoTarget::Direction(Direction::South))]
        );
        assert_eq!(engine.world().current_room_id(), core::roomId!("cellar"));
    }

    #[test]
    fn go_to_dead_end_then_back() {
        let mut engine = setup_engine();
        engine.handle_input("go north");
        engine.handle_input("go east");
        assert_eq!(engine.world().current_room_id(), core::roomId!("study"));

        let events = engine.handle_input("go west");
        assert_eq!(
            events,
            vec![Event::Went(GoTarget::Direction(Direction::West))]
        );
        assert_eq!(engine.world().current_room_id(), core::roomId!("corridor"));
    }

    #[test]
    fn go_invalid_direction_stays_in_room() {
        let mut engine = setup_engine();
        let events = engine.handle_input("go west");
        assert_eq!(
            events,
            vec![Event::WentExitNotFound(GoTarget::Direction(
                Direction::West
            ))]
        );
        assert_eq!(engine.world().current_room_id(), core::roomId!("cellar"));
    }

    #[test]
    fn go_north_shortcut() {
        let mut engine = setup_engine();
        let events = engine.handle_input("n");
        assert_eq!(
            events,
            vec![Event::Went(GoTarget::Direction(Direction::North))]
        );
        assert_eq!(engine.world().current_room_id(), core::roomId!("corridor"));
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
