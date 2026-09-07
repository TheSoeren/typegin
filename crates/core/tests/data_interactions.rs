//! Data-driven interactions: authored puzzle logic that ships in world data
//! (YAML) instead of Rust closures.
//!
//! This suite pins the schema of an `interactions:` block and the
//! dispatch/query semantics of roadmap item **#1 (data-driven game logic)**.
//!
//! ## Test data (lives in `crates/core/data/`, loaded via `include_str!`)
//!
//! * `data_interactions_items.yaml` — the world's objects (ids 101-110).
//! * `data_interactions_rooms.yaml` — the world's rooms (1 Cellar, 2
//!   Corridor, 3 Study).
//! * `interactions/<scenario>.yaml` — one snippet per distinct authored
//!   interaction block; each is a standalone YAML sequence that the helper
//!   appends under an `interactions:` key (mirroring the separate
//!   `data/interactions.yaml` file the terminal front-end loads).
//!
//! ## Schema (the `interactions` file, top level `interactions:` key)
//!
//! ```yaml
//! interactions:
//!   - verb: use               # Verb, lowercase ("look", "go", "examine",
//!     item: 101               #   "take", "drop", "use"); item is an object
//!     target:                 #   id, omitted to match any carried object
//!       kind: scene           # target: {kind: scene} or {object: <id>};
//!     condition:              #   omitted matches any target incl. self-use
//!       - room: 3             # conditions AND together; empty = always
//!       - player_holds: 102
//!       - exit_locked: east
//!       - exit_hidden: west
//!       - is_door: true       # target is (not) a door — door-ness is a
//!       - not: { room: 3 }    #   property, expressed as a condition; this
//!     effect:                 #   is *how* you target "any door" after the
//!       - emit: beat-name     #   kind taxonomy kept door out of the filters
//!       - take: 101           # Event::Took (no-op if object absent)
//!       - drop: 101           # Event::Dropped (no-op if absent)
//!       - unlock_exit: east   # Event::UnlockedExit (no-op if absent)
//!       - lock_exit: east     # silent
//!       - reveal_exit: north  # silent
//!       - hide_exit: east     # silent
//!       - reveal_object: 104  # silent
//!       - hide_object: 104    # silent
//! ```
//!
//! Unknown condition/effect/target nodes are a **load error** (`from_yaml`
//! returns `Err`), never silently ignored. Condition and effect lists are
//! optional (default empty).
//!
//! ## Semantics
//!
//! * `WorldData` carries a pub `interactions: Vec<InteractionData>` field
//!   populated from the interactions YAML; a missing key parses as empty.
//! * The default `on_take`/`on_drop`/`on_examine`/`on_use` consult these data
//!   interactions **before** `Rules::interactions()` closures and before the
//!   stock fallback spine. First match wins; a matching interaction fully
//!   replaces stock behaviour — its effects own the world mutation and the
//!   returned events.
//! * A custom `on_*` hook override bypasses data dispatch entirely.
//! * `Look`/`Go` are **query-only** verbs for data interactions (never
//!   dispatched), mirroring closure interactions.
//! * `interactions_for` reports data interactions first, then closure
//!   interactions, in declaration order.
//!
//! The public types are re-exported at the crate root (`InteractionData`,
//! `DataTarget`, `DataTargetKind`, `DataCondition`, `DataEffect`), and `Verb`
//! and `Direction` gain `serde::Deserialize` so the lowercase YAML spellings
//! (`use`, `east`, ...) load.
//!
//! Run with: `cd crates/core && cargo test --test data_interactions`.

mod common;

use core::ActionContext;
use core::{
    DataCondition, DataEffect, DataTarget, DataTargetKind, Direction, Event, GameEngine,
    Interaction, InteractionData, ObjectId, ObjectResolution, RoomId, Rules, TargetFilter, Verb,
    WorldData, WorldState,
};

/// The fixture world's item/scene objects (ids 101-110), from
/// `data_interactions_items.yaml`.
const ITEMS_YAML: &str = include_str!("../data/data_interactions_items.yaml");

/// The fixture world's rooms (1 Cellar, 2 Corridor, 3 Study), from
/// `data_interactions_rooms.yaml`.
///
/// Layout mirrors the multi-room fixture: room 1 -> north -> room 2 -> east ->
/// room 3. Room 3's exits: west (wooden door, open), north (secret passage,
/// hidden), east (oak door, locked, `gated_by` the iron key 101).
const ROOMS_YAML: &str = include_str!("../data/data_interactions_rooms.yaml");

/// The base fixture world (no interactions). The interactions string has no
/// `interactions` key, so it must parse as an empty list.
fn base_world() -> WorldData {
    WorldData::from_yaml(ITEMS_YAML, ROOMS_YAML, "{}").expect("base fixture parses")
}

/// The fixture world with an authored interaction snippet. `interactions` is a
/// standalone YAML sequence (as stored under `data/interactions/`), appended
/// under an `interactions:` key to mirror the separate interactions file.
fn world_with(interactions: &str) -> WorldData {
    let interactions_yaml = format!("interactions:\n{interactions}");
    WorldData::from_yaml(ITEMS_YAML, ROOMS_YAML, &interactions_yaml)
        .expect("fixture with authored interactions parses")
}

/// Opens the fixture world with authored interactions under the stock rules.
fn engine_with(interactions: &str) -> GameEngine {
    GameEngine::get(&world_with(interactions))
}

/// Walks from The Cellar into The Study via the corridor.
fn enter_study(engine: &mut GameEngine) {
    assert_eq!(
        engine.handle_input("go north"),
        vec![Event::Went(Direction::North)]
    );
    assert_eq!(
        engine.handle_input("go east"),
        vec![Event::Went(Direction::East)]
    );
    assert_eq!(engine.world().current_room_id(), RoomId::new(3));
}

/// Takes the iron key in The Cellar, then walks to The Study.
fn iron_key_in_study(engine: &mut GameEngine) {
    assert_eq!(
        engine.handle_input("take iron key"),
        vec![Event::Took {
            object_id: ObjectId::new(101),
            object: "iron key".to_string(),
        }]
    );
    enter_study(engine);
}

mod parse {
    use super::*;

    #[test]
    fn loads_interactions_from_the_interactions_file() {
        let world = world_with(include_str!("../data/interactions/parse_bundle.yaml"));
        assert_eq!(
            world.interactions,
            vec![
                InteractionData {
                    verb: Verb::Use,
                    item: Some(ObjectId::new(101)),
                    target: Some(DataTarget::Kind {
                        kind: DataTargetKind::Scene
                    }),
                    condition: vec![
                        DataCondition::Room { room: 3 },
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
                    item: Some(ObjectId::new(104)),
                    target: None,
                    condition: vec![],
                    effect: vec![
                        DataEffect::Take {
                            take: ObjectId::new(104)
                        },
                        DataEffect::Drop {
                            drop: ObjectId::new(104)
                        },
                        DataEffect::RevealObject {
                            reveal_object: ObjectId::new(104)
                        },
                    ],
                },
                InteractionData {
                    verb: Verb::Examine,
                    item: Some(ObjectId::new(103)),
                    target: Some(DataTarget::Object {
                        object: ObjectId::new(108)
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
            ITEMS_YAML,
            ROOMS_YAML,
            &format!(
                "interactions:\n{}",
                include_str!("../data/interactions/unknown_effect.yaml")
            ),
        );
        assert!(result.is_err());
    }
}

mod dispatch {
    use super::*;

    #[test]
    fn data_interaction_replaces_the_stock_unlock() {
        let mut engine = engine_with(include_str!("../data/interactions/unlock_and_emit.yaml"));
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
        let mut engine = engine_with(include_str!("../data/interactions/full_replace.yaml"));
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
    fn exact_object_target_gates_dispatch() {
        let mut engine = engine_with(include_str!(
            "../data/interactions/exact_object_target.yaml"
        ));
        iron_key_in_study(&mut engine);
        // 108 matches the authored target.
        assert_eq!(
            engine.handle_input("use iron key on wooden door"),
            vec![Event::Custom {
                name: "for-the-wooden-door".to_string()
            }]
        );
        // 110 does not: the stock unlock still fires for its gated_by key.
        assert_eq!(
            engine.handle_input("use iron key on oak door"),
            vec![Event::UnlockedExit {
                direction: Direction::East
            }]
        );
    }

    #[test]
    fn is_door_condition_matches_any_door() {
        let mut engine = engine_with(include_str!("../data/interactions/is_door.yaml"));
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
        let mut engine = engine_with(include_str!("../data/interactions/any_item.yaml"));
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
        let mut engine = engine_with(include_str!("../data/interactions/self_use.yaml"));
        assert_eq!(
            engine.handle_input("take rusty lamp"),
            vec![Event::Took {
                object_id: ObjectId::new(103),
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
    fn player_holds_condition_gates_query_and_dispatch() {
        let interactions = include_str!("../data/interactions/player_holds.yaml");
        // Holding only the iron key: gated off — the query lists nothing and
        // the stock unlock fires.
        let mut without_brass = engine_with(interactions);
        iron_key_in_study(&mut without_brass);
        assert!(
            without_brass
                .interactions_for(Some(ObjectId::new(101)), Some(ObjectId::new(110)))
                .is_empty()
        );
        assert_eq!(
            without_brass.handle_input("use iron key on oak door"),
            vec![Event::UnlockedExit {
                direction: Direction::East
            }]
        );

        // Holding both keys: the query lists the interaction and the dispatch
        // runs it — and since the effect owns the mutation, the door stays
        // locked.
        let mut with_brass = engine_with(interactions);
        assert_eq!(
            with_brass.handle_input("take iron key"),
            vec![Event::Took {
                object_id: ObjectId::new(101),
                object: "iron key".to_string(),
            }]
        );
        assert_eq!(
            with_brass.handle_input("take brass key"),
            vec![Event::Took {
                object_id: ObjectId::new(102),
                object: "brass key".to_string(),
            }]
        );
        enter_study(&mut with_brass);
        assert_eq!(
            with_brass
                .interactions_for(Some(ObjectId::new(101)), Some(ObjectId::new(110)))
                .len(),
            1
        );
        assert_eq!(
            with_brass.handle_input("use iron key on oak door"),
            vec![Event::Custom {
                name: "brass-holder-key-worked".to_string()
            }]
        );
        assert!(with_brass.world().is_exit_locked(Direction::East));
    }

    #[test]
    fn room_condition_gates_dispatch() {
        let interactions = include_str!("../data/interactions/room_gate.yaml");
        let mut in_cellar = engine_with(interactions);
        in_cellar.handle_input("take iron key");
        assert_eq!(
            in_cellar.handle_input("examine iron key"),
            vec![Event::Examined {
                object_id: ObjectId::new(101),
                object: "iron key".to_string(),
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
        let interactions = include_str!("../data/interactions/not_negation.yaml");
        let mut engine = engine_with(interactions);
        engine.handle_input("take iron key");
        enter_study(&mut engine);

        // Still locked: not(exit_locked) is false, so the stock examine runs.
        assert_eq!(
            engine.handle_input("examine oak door"),
            vec![Event::Examined {
                object_id: ObjectId::new(110),
                object: "oak door".to_string(),
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
        let mut engine = engine_with(include_str!("../data/interactions/first_wins.yaml"));
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
        let mut engine = engine_with(include_str!("../data/interactions/query_only_verbs.yaml"));
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
    use super::*;

    #[test]
    fn examine_effect_reveals_a_hidden_exit() {
        let mut engine = engine_with(include_str!("../data/interactions/reveal_exit.yaml"));
        enter_study(&mut engine);
        assert!(engine.world().is_exit_hidden(Direction::North));

        // The effect runs silently — no stock Examined, no Custom beat.
        assert_eq!(engine.handle_input("examine oak door"), vec![]);
        assert!(!engine.world().is_exit_hidden(Direction::North));
        assert_eq!(
            engine.world().resolve_target("secret passage"),
            ObjectResolution::Found(ObjectId::new(109))
        );
    }

    #[test]
    fn drop_effect_emits_then_drops() {
        let mut engine = engine_with(include_str!("../data/interactions/drop_emit.yaml"));
        assert_eq!(
            engine.handle_input("take iron key"),
            vec![Event::Took {
                object_id: ObjectId::new(101),
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
                    object_id: ObjectId::new(101),
                    object: "iron key".to_string(),
                },
            ]
        );
        assert!(!engine.world().player_holds(ObjectId::new(101)));
        assert!(
            engine
                .world()
                .room_object_names()
                .contains(&"iron key".to_string())
        );
    }

    #[test]
    fn lock_and_hide_effects_are_silent() {
        let mut engine = engine_with(include_str!("../data/interactions/lock_hide.yaml"));
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
}

mod precedence_and_rules {
    use super::*;

    struct ClosureRules(Vec<Interaction>);

    impl Rules for ClosureRules {
        fn interactions(&self) -> &[Interaction] {
            &self.0
        }
    }

    #[test]
    fn data_interactions_run_before_rules_interactions() {
        let world = world_with(include_str!("../data/interactions/data_wins.yaml"));
        let closure = vec![Interaction::build(
            Verb::Use,
            Some(ObjectId::new(101)),
            TargetFilter::Scene,
            Some(Box::new(|world: &WorldState, context: &ActionContext| {
                context.target.is_some_and(|id| world.object_is_door(id))
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
                _target_resolution: ObjectResolution,
            ) -> Vec<Event> {
                vec![Event::Custom {
                    name: "custom-hook".to_string(),
                }]
            }
        }

        let mut engine = GameEngine::get_with_rules(
            &world_with(include_str!("../data/interactions/data_wins.yaml")),
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
        let mut engine = engine_with(include_str!("../data/interactions/query_use.yaml"));
        iron_key_in_study(&mut engine);
        let listed = engine.interactions_for(Some(ObjectId::new(101)), Some(ObjectId::new(110)));
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].verb(), Verb::Use);
        // The wrong carried object does not match.
        assert!(
            engine
                .interactions_for(Some(ObjectId::new(102)), Some(ObjectId::new(110)))
                .is_empty()
        );
    }

    #[test]
    fn query_lists_data_first_then_rules() {
        let world = world_with(include_str!("../data/interactions/query_use.yaml"));
        let closure = vec![Interaction::build(
            Verb::Use,
            None,
            TargetFilter::Scene,
            Some(Box::new(|world: &WorldState, context: &ActionContext| {
                context.target.is_some_and(|id| world.object_is_door(id))
            })),
            Box::new(|_world: &mut WorldState, _context: &ActionContext| Vec::new()),
        )];
        let mut engine = GameEngine::get_with_rules(&world, ClosureRules(closure));
        iron_key_in_study(&mut engine);

        // The data interaction (item 101) is listed before the item-agnostic
        // closure interaction.
        let listed = engine.interactions_for(Some(ObjectId::new(101)), Some(ObjectId::new(110)));
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].item(), Some(ObjectId::new(101)));
        assert_eq!(listed[1].item(), None);
    }
}
