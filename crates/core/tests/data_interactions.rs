mod common;

use core::ActionContext;
use core::{
    DataCondition, DataEffect, DataTarget, DataTargetKind, Direction, Event, GameEngine,
    Interaction, InteractionData, ObjectId, ObjectResolution, RoomId, Rules, Target, TargetFilter,
    Verb, WorldData, WorldDataError, WorldState,
};

use common::{
    KEYED_ITEMS_YAML as ITEMS_YAML, KEYED_ROOMS_YAML as ROOMS_YAML, base_world,
    engine_with_interactions as engine_with, enter_study, world_with_interactions as world_with,
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
        let world = world_with(include_str!("fixtures/interactions/parse_bundle.yaml"));
        assert_eq!(
            world.interactions,
            vec![
                InteractionData {
                    verb: Verb::Use,
                    item: Some(ObjectId::new("iron-key")),
                    target: Some(DataTarget::Kind {
                        kind: DataTargetKind::Scene
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
        let result = WorldData::from_yaml(
            "{}",
            ITEMS_YAML,
            ROOMS_YAML,
            &format!(
                "interactions:\n{}",
                include_str!("fixtures/interactions/unknown_effect.yaml")
            ),
            "{}",
        );
        assert!(result.is_err());
    }

    #[test]
    fn grant_and_discard_effects_load() {
        let world = world_with(include_str!("fixtures/interactions/grant_and_discard.yaml"));
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
            let result = WorldData::from_yaml(
                "{}",
                ITEMS_YAML,
                ROOMS_YAML,
                &format!(
                    "interactions:\n- verb: use\n  item: iron-key\n  effect:\n    - {effect}: ghost-item\n"
                ),
                "{}",
            );
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
        let mut engine = engine_with(include_str!("fixtures/interactions/unlock_and_emit.yaml"));
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
        let mut engine = engine_with(include_str!("fixtures/interactions/full_replace.yaml"));
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
        let mut engine = engine_with(include_str!("fixtures/interactions/is_door.yaml"));
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
        let mut engine = engine_with(include_str!("fixtures/interactions/any_item.yaml"));
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
        let mut engine = engine_with(include_str!("fixtures/interactions/self_use.yaml"));
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
        let mut engine = engine_with(include_str!(
            "fixtures/interactions/no_target_rejects_surplus_target.yaml"
        ));
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
        let interactions = include_str!("fixtures/interactions/room_gate.yaml");
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
        let interactions = include_str!("fixtures/interactions/not_negation.yaml");
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
        let mut engine = engine_with(include_str!("fixtures/interactions/first_wins.yaml"));
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
        let mut engine = engine_with(include_str!("fixtures/interactions/query_only_verbs.yaml"));
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
        let mut engine = engine_with(include_str!("fixtures/interactions/reveal_exit.yaml"));
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
        let mut engine = engine_with(include_str!("fixtures/interactions/drop_emit.yaml"));
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
        let mut engine = engine_with(include_str!("fixtures/interactions/lock_hide.yaml"));
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
        let mut engine = engine_with(include_str!("fixtures/interactions/grant_unplaced.yaml"));
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
        let mut engine = engine_with(include_str!("fixtures/interactions/grant_when_held.yaml"));
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
        let mut engine = engine_with(include_str!("fixtures/interactions/discard_carried.yaml"));
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
        let mut engine = engine_with(include_str!("fixtures/interactions/discard_missing.yaml"));
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

    struct ClosureRules(Vec<Interaction>);

    impl Rules for ClosureRules {
        fn interactions(&self) -> &[Interaction] {
            &self.0
        }
    }

    #[test]
    fn data_interactions_run_before_rules_interactions() {
        let world = world_with(include_str!("fixtures/interactions/data_wins.yaml"));
        let closure = vec![Interaction::build(
            Verb::Use,
            Some(ObjectId::new("iron-key")),
            TargetFilter::Scene,
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

        let mut engine = GameEngine::get_with_rules(
            &world_with(include_str!("fixtures/interactions/data_wins.yaml")),
            OverrideRules,
        );
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

    struct ClosureRules(Vec<Interaction>);

    impl Rules for ClosureRules {
        fn interactions(&self) -> &[Interaction] {
            &self.0
        }
    }

    #[test]
    fn query_reports_matching_data_interactions() {
        let mut engine = engine_with(include_str!("fixtures/interactions/query_use.yaml"));
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
        let world = world_with(include_str!("fixtures/interactions/query_use.yaml"));
        let closure = vec![Interaction::build(
            Verb::Use,
            None,
            TargetFilter::Scene,
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
        // item-agnostic closure interaction.
        let listed = engine.interactions_for(
            Some(ObjectId::new("iron-key")),
            Some(Target::Object(ObjectId::new("oak-door"))),
        );
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].item(), Some(ObjectId::new("iron-key")));
    }
}
