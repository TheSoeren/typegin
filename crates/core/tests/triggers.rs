//! Spec for the room-event / trigger system (AGENTS.md engine gap #3): a
//! `World` hook distinct from `Interaction`'s verb-object shape. A trigger
//! fires at most once, its conditions checked after *every* action (not a
//! specific verb), reusing the existing `DataCondition`/`DataEffect`
//! vocabulary from authored interactions.
//!
//! Dispatch contract under test:
//! - all triggers whose condition holds fire in one pass, in declaration
//!   order (not first-match-wins, unlike `Interaction` dispatch);
//! - the "ready" set for a pass is a snapshot taken before any trigger in
//!   that pass runs, so one trigger's effect cannot make another trigger in
//!   the same pass newly eligible — a chain resolves over multiple player
//!   turns, never within one;
//! - each trigger fires at most once, ever (no re-arm).

mod common;

use core::{Direction, Event, RoomId, TriggerData, TriggerId, WorldData, WorldDataError};

use common::{engine_with_triggers as engine_with, world_with_triggers as world_with};

mod parse {
    use core::{DataCondition, DataEffect};

    use super::*;

    #[test]
    fn loads_triggers_from_the_globals_file() {
        let world = world_with(include_str!("fixtures/triggers/parse_bundle.yaml"));
        assert_eq!(
            world.triggers,
            vec![
                TriggerData {
                    id: TriggerId::new("guard-alert"),
                    condition: vec![
                        DataCondition::Room {
                            room: RoomId::new("study")
                        },
                        DataCondition::Flag {
                            flag: "alarm-armed".to_string()
                        },
                    ],
                    effect: vec![
                        DataEffect::Emit {
                            emit: "guard-alert".to_string()
                        },
                        DataEffect::SetFlag {
                            flag: "guard-alerted".to_string()
                        },
                    ],
                },
                TriggerData {
                    id: TriggerId::new("door-sensor"),
                    condition: vec![],
                    effect: vec![DataEffect::Emit {
                        emit: "sensor-tick".to_string()
                    }],
                },
            ]
        );
    }

    #[test]
    fn missing_triggers_key_parses_to_empty() {
        assert!(common::base_world().triggers.is_empty());
    }

    #[test]
    fn trigger_condition_referencing_an_unknown_room_is_a_validation_error() {
        let result = world_with_result(include_str!("fixtures/triggers/unknown_room.yaml"));
        assert!(matches!(result, Err(WorldDataError::Validation(_))));
    }

    #[test]
    fn duplicate_trigger_ids_are_a_validation_error() {
        let result = world_with_result(include_str!("fixtures/triggers/duplicate_ids.yaml"));
        assert!(matches!(result, Err(WorldDataError::Validation(_))));
    }

    fn world_with_result(triggers: &str) -> Result<WorldData, WorldDataError> {
        let globals_yaml = format!("triggers:\n{triggers}");
        WorldData::from_yaml(
            &globals_yaml,
            common::KEYED_ITEMS_YAML,
            common::KEYED_ROOMS_YAML,
            "{}",
            "{}",
        )
    }
}

mod dispatch {
    use super::*;

    #[test]
    fn room_entry_trigger_fires_once_on_entering_the_room() {
        let mut engine = engine_with(include_str!("fixtures/triggers/room_entry_once.yaml"));

        // Walking into the corridor does not satisfy `room: study`.
        assert_eq!(
            engine.handle_input("go north"),
            vec![Event::Went(Direction::North)]
        );

        // The final step into the study fires the trigger, appended after
        // the stock `Went` event.
        assert_eq!(
            engine.handle_input("go east"),
            vec![
                Event::Went(Direction::East),
                Event::Custom {
                    name: "study-entered".to_string()
                },
            ]
        );

        // Leaving and re-entering does not re-fire: one-shot only.
        assert_eq!(
            engine.handle_input("go west"),
            vec![Event::Went(Direction::West)]
        );
        assert_eq!(
            engine.handle_input("go south"),
            vec![Event::Went(Direction::South)]
        );
        assert_eq!(
            engine.handle_input("go north"),
            vec![Event::Went(Direction::North)]
        );
        assert_eq!(
            engine.handle_input("go east"),
            vec![Event::Went(Direction::East)]
        );
    }

    #[test]
    fn trigger_fires_off_any_action_not_just_a_specific_verb() {
        let mut engine = engine_with(include_str!("fixtures/triggers/flag_transition.yaml"));

        // The flag is unset: a `Look` action (never routed through
        // `Interaction` dispatch) still checks triggers, and none fire.
        assert_eq!(engine.handle_input("look"), vec![Event::Looked]);

        // Set the flag directly on the world (standing in for whatever
        // authored effect would normally flip it) and act again with an
        // unrelated verb: the trigger fires because its condition now
        // holds, regardless of which action caused the check.
        engine.world_mut().set_flag("ac-off");
        assert_eq!(
            engine.handle_input("look"),
            vec![
                Event::Looked,
                Event::Custom {
                    name: "fan-still".to_string()
                },
            ]
        );

        // One-shot: does not fire again even though the flag is still set.
        assert_eq!(engine.handle_input("look"), vec![Event::Looked]);
    }

    #[test]
    fn all_triggers_ready_in_the_same_pass_fire_in_declaration_order() {
        let mut engine = engine_with(include_str!("fixtures/triggers/two_ready_same_turn.yaml"));
        engine.handle_input("go north");
        assert_eq!(
            engine.handle_input("go east"),
            vec![
                Event::Went(Direction::East),
                Event::Custom {
                    name: "first-beat".to_string()
                },
                Event::Custom {
                    name: "second-beat".to_string()
                },
            ]
        );
    }

    #[test]
    fn a_trigger_firing_does_not_cascade_into_a_newly_ready_trigger_the_same_turn() {
        let mut engine = engine_with(include_str!("fixtures/triggers/cascade_single_pass.yaml"));
        engine.handle_input("go north");

        // Entering the study fires `enter-sets-flag`, which sets
        // `chain-flag`. `depends-on-chain-flag` is also newly satisfied by
        // that, but the ready set for this pass was already snapshotted
        // before `enter-sets-flag` ran, so it does not fire this turn.
        assert_eq!(
            engine.handle_input("go east"),
            vec![
                Event::Went(Direction::East),
                Event::FlagSet {
                    flag: "chain-flag".to_string()
                },
            ]
        );

        // The next action re-checks triggers against the now-current state:
        // `depends-on-chain-flag` fires.
        assert_eq!(
            engine.handle_input("look"),
            vec![
                Event::Looked,
                Event::Custom {
                    name: "chain-fired".to_string()
                },
            ]
        );
    }
}
