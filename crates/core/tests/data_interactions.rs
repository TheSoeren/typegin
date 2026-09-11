mod common;

use core::ActionContext;
use core::{
    DataCondition, DataEffect, DataTarget, Direction, Event, GameEngine, Interaction,
    InteractionData, ObjectId, ObjectResolution, RoomId, Rules, Target, TargetFilter, TargetKind,
    Verb, WorldData, WorldDataError, WorldState,
};

use common::{
    KEYED_WORLD_YAML, base_world, engine_with_interactions as engine_with, enter_study, merge_yaml,
    world_with_interactions as world_with,
};

/// Takes the iron key in The Cellar, then walks to The Study.
fn iron_key_in_study(engine: &mut GameEngine) {
    assert_eq!(
        engine.handle_input("take iron key"),
        vec![Event::Took {
            object_id: ObjectId::new("iron-key"),
            object: "iron key".to_string(),
        }]
    );
    enter_study(engine);
}

mod parse {
    use super::*;

    #[test]
    fn loads_interactions_from_the_interactions_file() {
        let world = world_with(
            r"- verb: use
  item: iron-key
  target:
    kind: scene
  condition:
    - room: study
    - exit_locked: east
    - is_door: true
  effect:
    - emit: door-unlocked
    - unlock_exit: east
- verb: take
  item: stale-bread
  effect:
    - take: stale-bread
    - drop: stale-bread
    - reveal_object: stale-bread
- verb: examine
  target:
    object: rusty-lamp",
        );
        assert_eq!(
            world.interactions,
            vec![
                InteractionData {
                    verb: Verb::Use,
                    item: Some(ObjectId::new("iron-key")),
                    target: Some(DataTarget::Kind {
                        kind: TargetKind::Scene
                    }),
                    condition: vec![
                        DataCondition::Room {
                            room: RoomId::new("study")
                        },
                        DataCondition::ExitLocked {
                            exit_locked: Direction::East
                        },
                        DataCondition::IsDoor { is_door: true },
                    ],
                    effect: vec![
                        DataEffect::Emit {
                            emit: "door-unlocked".to_string()
                        },
                        DataEffect::UnlockExit {
                            unlock_exit: Direction::East
                        },
                    ],
                },
                InteractionData {
                    verb: Verb::Take,
                    item: Some(ObjectId::new("stale-bread")),
                    target: None,
                    condition: vec![],
                    effect: vec![
                        DataEffect::Take {
                            take: ObjectId::new("stale-bread")
                        },
                        DataEffect::Drop {
                            drop: ObjectId::new("stale-bread")
                        },
                        DataEffect::RevealObject {
                            reveal_object: ObjectId::new("stale-bread")
                        },
                    ],
                },
                InteractionData {
                    verb: Verb::Examine,
                    item: None,
                    target: Some(DataTarget::Object {
                        object: ObjectId::new("rusty-lamp")
                    }),
                    condition: vec![],
                    effect: vec![],
                },
            ]
        );
    }

    #[test]
    fn missing_interactions_key_parses_to_empty() {
        assert!(base_world().interactions.is_empty());
    }

    #[test]
    fn unknown_effect_node_is_a_parse_error() {
        let interactions_yaml = "interactions:\n- verb: examine\n  target:\n    object: stale-bread\n  effect:\n    - explode: true\n";
        let result = WorldData::from_yaml(&merge_yaml(&[KEYED_WORLD_YAML, interactions_yaml]));
        assert!(result.is_err());
    }

    #[test]
    fn grant_and_discard_effects_load() {
        let world = world_with(
            r"- verb: use
  item: iron-key
  effect:
    - grant: rusty-nail
    - discard: rusty-nail",
        );
        let interaction = &world.interactions[0];
        assert_eq!(
            interaction.effect,
            vec![
                DataEffect::Grant {
                    grant: ObjectId::new("rusty-nail"),
                },
                DataEffect::Discard {
                    discard: ObjectId::new("rusty-nail"),
                },
            ]
        );
    }

    #[test]
    fn grant_or_discard_referencing_an_unknown_object_is_a_validation_error() {
        for effect in ["grant", "discard"] {
            let interactions_yaml = format!(
                "interactions:\n- verb: use\n  item: iron-key\n  effect:\n    - {effect}: ghost-item\n"
            );
            let result = WorldData::from_yaml(&merge_yaml(&[KEYED_WORLD_YAML, &interactions_yaml]));
            assert!(
                matches!(result, Err(WorldDataError::Validation(_))),
                "unknown `{effect}` target should be a validation error, got {result:?}"
            );
        }
    }
}

mod dispatch {
    use super::*;

    #[test]
    fn data_interaction_replaces_the_stock_unlock() {
        let mut engine = engine_with(
            r"- verb: use
  item: iron-key
  target:
    kind: scene
  condition:
    - is_door: true
  effect:
    - unlock_exit: east
    - emit: door-unlocked",
        );
        iron_key_in_study(&mut engine);
        assert!(engine.world().is_exit_locked(Direction::East));
        assert_eq!(
            engine.handle_input("use iron key on oak door"),
            vec![
                Event::UnlockedExit {
                    direction: Direction::East
                },
                Event::Custom {
                    name: "door-unlocked".to_string()
                },
            ]
        );
        assert!(!engine.world().is_exit_locked(Direction::East));
    }

    #[test]
    fn authored_beat_fully_replaces_the_stock_beat() {
        let mut engine = engine_with(
            r"- verb: use
  item: iron-key
  target:
    object: oak-door
  effect:
    - emit: key-does-not-work",
        );
        iron_key_in_study(&mut engine);
        assert_eq!(
            engine.handle_input("use iron key on oak door"),
            vec![Event::Custom {
                name: "key-does-not-work".to_string()
            }]
        );
        // The lock stays: the stock UnlockedExit never ran.
        assert!(engine.world().is_exit_locked(Direction::East));
    }

    #[test]
    fn is_door_condition_matches_any_door() {
        let mut engine = engine_with(
            r"- verb: use
  item: iron-key
  target:
    kind: scene
  condition:
    - room: study
    - is_door: true
  effect:
    - emit: used-on-a-door",
        );
        iron_key_in_study(&mut engine);
        assert_eq!(
            engine.handle_input("use iron key on oak door"),
            vec![Event::Custom {
                name: "used-on-a-door".to_string()
            }]
        );
        assert_eq!(
            engine.handle_input("use iron key on wooden door"),
            vec![Event::Custom {
                name: "used-on-a-door".to_string()
            }]
        );
    }

    #[test]
    fn omitted_item_matches_any_carried_object() {
        let mut engine = engine_with(
            r"- verb: use
  target:
    object: oak-door
  effect:
    - emit: anything-used-on-the-oak",
        );
        iron_key_in_study(&mut engine);
        assert_eq!(
            engine.handle_input("use iron key on oak door"),
            vec![Event::Custom {
                name: "anything-used-on-the-oak".to_string()
            }]
        );
    }

    #[test]
    fn omitted_target_matches_self_use() {
        let mut engine = engine_with(
            r"- verb: use
  item: rusty-lamp
  effect:
    - emit: lamp-considered",
        );
        assert_eq!(
            engine.handle_input("take rusty lamp"),
            vec![Event::Took {
                object_id: ObjectId::new("rusty-lamp"),
                object: "rusty lamp".to_string(),
            }]
        );
        assert_eq!(
            engine.handle_input("use rusty lamp"),
            vec![Event::Custom {
                name: "lamp-considered".to_string()
            }]
        );
    }

    #[test]
    fn no_target_interaction_rejects_surplus_target() {
        let mut engine = engine_with(
            r"- verb: use
  item: rusty-lamp
  effect:
    - emit: lamp-considered",
        );
        // Take the lamp so it's in inventory.
        assert_eq!(
            engine.handle_input("take rusty lamp"),
            vec![Event::Took {
                object_id: ObjectId::new("rusty-lamp"),
                object: "rusty lamp".to_string(),
            }]
        );
        // Use with no target: matches the interaction.
        assert_eq!(
            engine.handle_input("use rusty lamp"),
            vec![Event::Custom {
                name: "lamp-considered".to_string()
            }]
        );
        // Use ON a target: must NOT match the no-target interaction.
        // The engine should fall through to the stock "Used" event.
        assert_eq!(
            engine.handle_input("use rusty lamp on iron key"),
            vec![Event::Used {
                object_id: ObjectId::new("rusty-lamp"),
                object: "rusty lamp".to_string(),
                target_id: Some(Target::Object(ObjectId::new("iron-key"))),
                target: Some("iron key".to_string()),
            }]
        );
    }

    #[test]
    fn room_condition_gates_dispatch() {
        let interactions = r"- verb: examine
  target:
    object: iron-key
  condition:
    - room: study
  effect:
    - emit: key-in-study";
        let mut in_cellar = engine_with(interactions);
        in_cellar.handle_input("take iron key");
        assert_eq!(
            in_cellar.handle_input("examine iron key"),
            vec![Event::Examined {
                target: Target::Object(ObjectId::new("iron-key")),
                target_name: "iron key".to_string(),
            }]
        );

        let mut in_study = engine_with(interactions);
        in_study.handle_input("take iron key");
        enter_study(&mut in_study);
        assert_eq!(
            in_study.handle_input("examine iron key"),
            vec![Event::Custom {
                name: "key-in-study".to_string()
            }]
        );
    }

    #[test]
    fn not_condition_negates() {
        let interactions = r"- verb: examine
  target:
    object: oak-door
  condition:
    - not:
        exit_locked: east
  effect:
    - emit: door-checked-twice

- verb: use
  item: iron-key
  target:
    object: oak-door
  effect:
    - unlock_exit: east";
        let mut engine = engine_with(interactions);
        engine.handle_input("take iron key");
        enter_study(&mut engine);

        // Still locked: not(exit_locked) is false, so the stock examine runs.
        assert_eq!(
            engine.handle_input("examine oak door"),
            vec![Event::Examined {
                target: Target::Object(ObjectId::new("oak-door")),
                target_name: "oak door".to_string(),
            }]
        );

        // Unlock, and the beat fires.
        assert_eq!(
            engine.handle_input("use iron key on oak door"),
            vec![Event::UnlockedExit {
                direction: Direction::East
            }]
        );
        assert_eq!(
            engine.handle_input("examine oak door"),
            vec![Event::Custom {
                name: "door-checked-twice".to_string()
            }]
        );
    }

    #[test]
    fn first_declared_interaction_wins() {
        let mut engine = engine_with(
            r"- verb: use
  item: iron-key
  target:
    kind: scene
  condition:
    - is_door: true
  effect:
    - emit: first-wins
- verb: use
  item: iron-key
  target:
    kind: scene
  condition:
    - is_door: true
  effect:
    - emit: shadowed",
        );
        iron_key_in_study(&mut engine);
        assert_eq!(
            engine.handle_input("use iron key on oak door"),
            vec![Event::Custom {
                name: "first-wins".to_string()
            }]
        );
    }

    #[test]
    fn look_and_go_are_query_only() {
        let mut engine = engine_with(
            r"- verb: look
  effect:
    - emit: never-run-look
- verb: go
  effect:
    - emit: never-run-go",
        );
        assert_eq!(engine.handle_input("look"), vec![Event::Looked]);
        assert_eq!(
            engine.handle_input("go north"),
            vec![Event::Went(Direction::North)]
        );
        // But the query still reports both verbs, in declaration order.
        let listed = engine.interactions_for(None, None);
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].verb(), Verb::Look);
        assert_eq!(listed[1].verb(), Verb::Go);
    }
}

mod effects {
    use core::world::object::TargetResolution;

    use super::*;

    #[test]
    fn examine_effect_reveals_a_hidden_exit() {
        let mut engine = engine_with(
            r"- verb: examine
  target:
    object: oak-door
  effect:
    - reveal_exit: north",
        );
        enter_study(&mut engine);
        assert!(engine.world().is_exit_hidden(Direction::North));

        // The effect runs silently — no stock Examined, no Custom beat.
        assert_eq!(engine.handle_input("examine oak door"), vec![]);
        assert!(!engine.world().is_exit_hidden(Direction::North));
        assert_eq!(
            engine.world().resolve_target("secret passage"),
            TargetResolution::Found(Target::Object(ObjectId::new("secret-passage")))
        );
    }

    #[test]
    fn drop_effect_emits_then_drops() {
        let mut engine = engine_with(
            r"- verb: drop
  item: iron-key
  effect:
    - emit: key-discarded
    - drop: iron-key",
        );
        assert_eq!(
            engine.handle_input("take iron key"),
            vec![Event::Took {
                object_id: ObjectId::new("iron-key"),
                object: "iron key".to_string(),
            }]
        );
        assert_eq!(
            engine.handle_input("drop iron key"),
            vec![
                Event::Custom {
                    name: "key-discarded".to_string()
                },
                Event::Dropped {
                    object_id: ObjectId::new("iron-key"),
                    object: "iron key".to_string(),
                },
            ]
        );
        assert!(!engine.world().player_holds(&ObjectId::new("iron-key")));
        assert!(
            engine
                .world()
                .room_object_names()
                .contains(&"iron key".to_string())
        );
    }

    #[test]
    fn lock_and_hide_effects_are_silent() {
        let mut engine = engine_with(
            r"- verb: use
  item: iron-key
  target:
    kind: scene
  condition:
    - is_door: true
  effect:
    - lock_exit: east
    - hide_exit: east",
        );
        iron_key_in_study(&mut engine);
        engine.world_mut().unlock_exit(Direction::East);
        assert!(!engine.world().is_exit_locked(Direction::East));

        assert_eq!(engine.handle_input("use iron key on oak door"), vec![]);
        assert!(engine.world().is_exit_locked(Direction::East));
        assert!(engine.world().is_exit_hidden(Direction::East));
        assert_eq!(
            engine.handle_input("go east"),
            vec![Event::WentExitHidden(Direction::East)]
        );
    }

    #[test]
    fn grant_materialises_an_item_placed_nowhere_in_the_world() {
        let mut engine = engine_with(
            r"- verb: use
  item: iron-key
  target:
    kind: scene
  effect:
    - grant: rusty-nail",
        );
        assert_eq!(
            engine.handle_input("take iron key"),
            vec![Event::Took {
                object_id: ObjectId::new("iron-key"),
                object: "iron key".to_string(),
            }]
        );
        // rusty-nail is declared in data but sits in no room: it is granted
        // into the inventory out of nowhere.
        assert_eq!(
            engine.handle_input("use iron key on cellar stairs"),
            vec![Event::Granted {
                object_id: ObjectId::new("rusty-nail"),
                object: "rusty nail".to_string(),
            }]
        );
        assert!(engine.world().player_holds(&ObjectId::new("rusty-nail")));
        assert!(
            !engine
                .world()
                .room_object_names()
                .contains(&"rusty nail".to_string())
        );
    }

    #[test]
    fn grant_is_a_no_op_when_the_player_already_holds_the_item() {
        let mut engine = engine_with(
            r"- verb: use
  item: brass-key
  target:
    kind: scene
  effect:
    - grant: iron-key
    - emit: done",
        );
        assert_eq!(
            engine.handle_input("take iron key"),
            vec![Event::Took {
                object_id: ObjectId::new("iron-key"),
                object: "iron key".to_string(),
            }]
        );
        assert_eq!(
            engine.handle_input("take brass key"),
            vec![Event::Took {
                object_id: ObjectId::new("brass-key"),
                object: "brass key".to_string(),
            }]
        );
        // The grant of the already-held iron key is a silent no-op; only the
        // explicit emit produces a beat.
        assert_eq!(
            engine.handle_input("use brass key on cellar stairs"),
            vec![Event::Custom {
                name: "done".to_string(),
            }]
        );
        assert_eq!(
            engine
                .world()
                .player_object_names()
                .iter()
                .filter(|name| name.as_str() == "iron key")
                .count(),
            1
        );
    }

    #[test]
    fn discard_removes_a_carried_item_without_dropping_it_in_the_room() {
        let mut engine = engine_with(
            r"- verb: use
  item: iron-key
  target:
    kind: scene
  effect:
    - discard: iron-key",
        );
        assert_eq!(
            engine.handle_input("take iron key"),
            vec![Event::Took {
                object_id: ObjectId::new("iron-key"),
                object: "iron key".to_string(),
            }]
        );
        assert_eq!(
            engine.handle_input("use iron key on cellar stairs"),
            vec![Event::Discarded {
                object_id: ObjectId::new("iron-key"),
                object: "iron key".to_string(),
            }]
        );
        assert!(!engine.world().player_holds(&ObjectId::new("iron-key")));
        assert!(
            !engine
                .world()
                .room_object_names()
                .contains(&"iron key".to_string())
        );
        // The item exists nowhere now: it can be neither be dropped nor used.
        assert_eq!(
            engine.handle_input("drop iron key"),
            vec![Event::DroppedObjectNotFound {
                object: "iron key".to_string(),
            }]
        );
    }

    #[test]
    fn discard_is_a_no_op_when_the_player_does_not_hold_the_item() {
        let mut engine = engine_with(
            r"- verb: use
  item: brass-key
  target:
    kind: scene
  effect:
    - discard: iron-key",
        );
        assert_eq!(
            engine.handle_input("take brass key"),
            vec![Event::Took {
                object_id: ObjectId::new("brass-key"),
                object: "brass key".to_string(),
            }]
        );
        // discard of an item that is not carried: silent, nothing changes.
        assert_eq!(
            engine.handle_input("use brass key on cellar stairs"),
            vec![]
        );
        // iron-key stayed in the cellar; it was not consumed from the room.
        assert_eq!(
            engine.world().resolve_target("iron key"),
            TargetResolution::Found(Target::Object(ObjectId::new("iron-key")))
        );
    }
}

mod precedence_and_rules {
    use core::world::object::TargetResolution;

    use super::*;

    /// Used by both tests in this module.
    const DATA_WINS_YAML: &str = r"- verb: use
  item: iron-key
  target:
    kind: scene
  condition:
    - is_door: true
  effect:
    - emit: data-wins";

    struct ClosureRules(Vec<Interaction>);

    impl Rules for ClosureRules {
        fn interactions(&self) -> &[Interaction] {
            &self.0
        }
    }

    #[test]
    fn data_interactions_run_before_rules_interactions() {
        let world = world_with(DATA_WINS_YAML);
        let closure = vec![Interaction::build(
            Verb::Use,
            Some(ObjectId::new("iron-key")),
            TargetFilter::Kind(TargetKind::Scene),
            Some(Box::new(|world: &WorldState, context: &ActionContext| {
                context
                    .target_object()
                    .is_some_and(|id| world.object_is_door(id))
            })),
            Box::new(|_world: &mut WorldState, _context: &ActionContext| {
                vec![Event::Custom {
                    name: "rules-wins".to_string(),
                }]
            }),
        )];
        let mut engine = GameEngine::get_with_rules(&world, ClosureRules(closure));
        iron_key_in_study(&mut engine);
        assert_eq!(
            engine.handle_input("use iron key on oak door"),
            vec![Event::Custom {
                name: "data-wins".to_string()
            }]
        );
    }

    #[test]
    fn custom_on_use_overrides_data_dispatch() {
        struct OverrideRules;

        impl Rules for OverrideRules {
            fn on_use(
                &mut self,
                _world: &mut WorldState,
                _item: &str,
                _target: Option<&str>,
                _item_resolution: ObjectResolution,
                _target_resolution: TargetResolution,
            ) -> Vec<Event> {
                vec![Event::Custom {
                    name: "custom-hook".to_string(),
                }]
            }
        }

        let mut engine = GameEngine::get_with_rules(&world_with(DATA_WINS_YAML), OverrideRules);
        iron_key_in_study(&mut engine);
        assert_eq!(
            engine.handle_input("use iron key on oak door"),
            vec![Event::Custom {
                name: "custom-hook".to_string()
            }]
        );
    }
}

mod interactions_for {
    use super::*;

    /// Used by both tests in this module.
    const QUERY_USE_YAML: &str = r"- verb: use
  item: iron-key
  target:
    kind: scene
  condition:
    - is_door: true
  effect:
    - emit: unlock-flavor";

    struct ClosureRules(Vec<Interaction>);

    impl Rules for ClosureRules {
        fn interactions(&self) -> &[Interaction] {
            &self.0
        }
    }

    #[test]
    fn query_reports_matching_data_interactions() {
        let mut engine = engine_with(QUERY_USE_YAML);
        iron_key_in_study(&mut engine);
        let listed = engine.interactions_for(
            Some(ObjectId::new("iron-key")),
            Some(Target::Object(ObjectId::new("oak-door"))),
        );
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].verb(), Verb::Use);
        // The wrong carried object does not match.
        assert!(
            engine
                .interactions_for(
                    Some(ObjectId::new("brass-key")),
                    Some(Target::Object(ObjectId::new("oak-door")))
                )
                .is_empty()
        );
    }

    #[test]
    fn query_lists_data_first_then_rules() {
        let world = world_with(QUERY_USE_YAML);
        let closure = vec![Interaction::build(
            Verb::Use,
            None,
            TargetFilter::Kind(TargetKind::Scene),
            Some(Box::new(|world: &WorldState, context: &ActionContext| {
                context
                    .target_object()
                    .is_some_and(|id| world.object_is_door(id))
            })),
            Box::new(|_world: &mut WorldState, _context: &ActionContext| Vec::new()),
        )];
        let mut engine = GameEngine::get_with_rules(&world, ClosureRules(closure));
        iron_key_in_study(&mut engine);

        // The data interaction (keyed to iron-key) is listed before the
        // item-agnostic closure interaction, and both are reported: the
        // closure's `item: None` matches any carried item, including this one.
        let listed = engine.interactions_for(
            Some(ObjectId::new("iron-key")),
            Some(Target::Object(ObjectId::new("oak-door"))),
        );
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].item(), Some(ObjectId::new("iron-key")));
        assert_eq!(listed[1].item(), None);
    }
}
