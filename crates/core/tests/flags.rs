//! Flags / global quest / causal state.
//!
//! A first-class flags model on [`WorldState`] so "when X happens then Y" and
//! conditional states are declarative, not bolted on via closures. Flags are
//! simple string-keyed boolean state: set by interaction effects, queried by
//! interaction conditions.
//!
//! ## Schema additions
//!
//! ### Data condition
//!
//! ```yaml
//! condition:
//!   - flag: quest-started    # true when the flag is set
//!   - not:
//!       flag: door-seen      # true when the flag is NOT set
//! ```
//!
//! ### Data effects
//!
//! ```yaml
//! effect:
//!   - set_flag: phase-two    # sets the flag (silent)
//!   - clear_flag: phase-one  # clears the flag (silent)
//! ```
//!
//! ### Initial flags
//!
//! ```yaml
//! flags:                      # top-level key in world data
//!   - game-started
//!   - intro-seen
//! ```
//!
//! Initial flags are set on [`WorldState`] construction and are immediately
//! queryable.
//!
//! ## Semantics
//!
//! * Flag names are free-form strings; no validation against declared data.
//! * `set_flag` / `clear_flag` effects are silent (no event emitted). Pair
//!   with an `emit` effect if you need a visible beat.
//! * Flags are mutable world state — the only way to change them is through
//!   interaction effects (or custom `Rules` closures via `WorldState`).
//! * `interactions_for` re-evaluates flag conditions live, so setting a flag
//!   immediately makes gated interactions appear (and vice versa).
//!
//! Run with: `cargo test --test flags`.

mod common;

use core::{DataCondition, DataEffect, Direction, Event, GameEngine, ObjectId, Target, WorldData};

use common::{
    KEYED_GLOBALS_YAML as GLOBALS_YAML, KEYED_ITEMS_YAML as ITEMS_YAML,
    KEYED_ROOMS_YAML as ROOMS_YAML, base_world, engine_with_interactions as engine_with,
    enter_study, world_data_with_initial_flags, world_with_interactions as world_with,
};

// ---------------------------------------------------------------------------
// WorldState API
// ---------------------------------------------------------------------------

mod world_state_api {
    use super::*;

    #[test]
    fn flags_start_empty() {
        let engine = GameEngine::get(&base_world());
        assert!(!engine.world().has_flag("anything"));
    }

    #[test]
    fn set_flag_makes_has_flag_true() {
        let mut engine = GameEngine::get(&base_world());
        assert!(!engine.world().has_flag("quest-started"));
        engine.world_mut().set_flag("quest-started");
        assert!(engine.world().has_flag("quest-started"));
    }

    #[test]
    fn clear_flag_makes_has_flag_false() {
        let mut engine = GameEngine::get(&base_world());
        engine.world_mut().set_flag("temp");
        assert!(engine.world().has_flag("temp"));
        engine.world_mut().clear_flag("temp");
        assert!(!engine.world().has_flag("temp"));
    }

    #[test]
    fn clearing_a_missing_flag_is_a_noop() {
        let mut engine = GameEngine::get(&base_world());
        engine.world_mut().clear_flag("nonexistent");
        assert!(!engine.world().has_flag("nonexistent"));
    }
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

mod parse {
    use super::*;

    #[test]
    fn flag_condition_parses() {
        let world = world_with(include_str!("fixtures/interactions/flag_condition.yaml"));
        let interaction = &world.interactions[0];
        assert_eq!(
            interaction.condition,
            vec![DataCondition::Flag {
                flag: "quest-started".to_string()
            }]
        );
    }

    #[test]
    fn set_flag_effect_parses() {
        let world = world_with(include_str!("fixtures/interactions/set_flag_effect.yaml"));
        let interaction = &world.interactions[0];
        assert!(interaction.effect.contains(&DataEffect::SetFlag {
            flag: "door-unlocked-flag".to_string()
        }));
    }

    #[test]
    fn clear_flag_effect_parses() {
        let world = world_with(include_str!("fixtures/interactions/clear_flag_effect.yaml"));
        let interaction = &world.interactions[0];
        assert!(interaction.effect.contains(&DataEffect::ClearFlag {
            flag: "temp-flag".to_string()
        }));
    }

    #[test]
    fn flag_in_not_condition_parses() {
        let world = world_with(include_str!("fixtures/interactions/flag_with_not.yaml"));
        let interaction = &world.interactions[0];
        assert_eq!(
            interaction.condition,
            vec![DataCondition::Not {
                not: Box::new(DataCondition::Flag {
                    flag: "door-seen".to_string()
                })
            }]
        );
    }

    #[test]
    fn initial_flags_field_parses() {
        let mut data = base_world();
        data.flags = vec!["flag-a".to_string(), "flag-b".to_string()];
        assert_eq!(data.flags, vec!["flag-a", "flag-b"]);
    }
}

// ---------------------------------------------------------------------------
// Dispatch: flag condition gates interactions
// ---------------------------------------------------------------------------

mod dispatch {
    use super::*;

    #[test]
    fn flag_condition_blocks_when_flag_not_set() {
        let mut engine = engine_with(include_str!("fixtures/interactions/flag_condition.yaml"));
        assert_eq!(
            engine.handle_input("take iron key"),
            vec![Event::Took {
                object_id: ObjectId::new("iron-key"),
                object: "iron key".to_string(),
            }]
        );
        // flag not set → stock examine fires
        assert_eq!(
            engine.handle_input("examine iron key"),
            vec![Event::Examined {
                target: Target::Object(ObjectId::new("iron-key")),
                target_name: "iron key".to_string(),
            }]
        );
    }

    #[test]
    fn flag_condition_allows_when_flag_is_set() {
        let mut engine = engine_with(include_str!("fixtures/interactions/flag_condition.yaml"));
        assert_eq!(
            engine.handle_input("take iron key"),
            vec![Event::Took {
                object_id: ObjectId::new("iron-key"),
                object: "iron key".to_string(),
            }]
        );
        engine.world_mut().set_flag("quest-started");
        assert_eq!(
            engine.handle_input("examine iron key"),
            vec![Event::Custom {
                name: "quest-item-recognized".to_string()
            }]
        );
    }

    #[test]
    fn set_flag_effect_sets_the_flag() {
        let mut engine = engine_with(include_str!("fixtures/interactions/set_flag_effect.yaml"));
        assert!(!engine.world().has_flag("door-unlocked-flag"));
        // Use iron key on oak door (a scene/door object) to trigger the effect.
        // Walk to study first where the oak door lives.
        assert_eq!(
            engine.handle_input("take iron key"),
            vec![Event::Took {
                object_id: ObjectId::new("iron-key"),
                object: "iron key".to_string(),
            }]
        );
        enter_study(&mut engine);
        assert_eq!(
            engine.handle_input("use iron key on oak door"),
            vec![
                Event::FlagSet {
                    flag: "door-unlocked-flag".to_string()
                },
                Event::Custom {
                    name: "door-handled".to_string()
                },
            ]
        );
        assert!(engine.world().has_flag("door-unlocked-flag"));
    }

    #[test]
    fn clear_flag_effect_clears_the_flag() {
        let mut engine = engine_with(include_str!("fixtures/interactions/clear_flag_effect.yaml"));
        engine.world_mut().set_flag("temp-flag");
        assert_eq!(
            engine.handle_input("take iron key"),
            vec![Event::Took {
                object_id: ObjectId::new("iron-key"),
                object: "iron key".to_string(),
            }]
        );
        assert!(engine.world().has_flag("temp-flag"));
        assert_eq!(
            engine.handle_input("examine iron key"),
            vec![
                Event::FlagCleared {
                    flag: "temp-flag".to_string()
                },
                Event::Custom {
                    name: "flag-cleared".to_string()
                },
            ]
        );
        assert!(!engine.world().has_flag("temp-flag"));
    }

    #[test]
    fn not_flag_negates() {
        let mut engine = engine_with(include_str!("fixtures/interactions/flag_with_not.yaml"));
        assert_eq!(
            engine.handle_input("take iron key"),
            vec![Event::Took {
                object_id: ObjectId::new("iron-key"),
                object: "iron key".to_string(),
            }]
        );
        // flag not set → not(flag) is true → custom fires
        assert_eq!(
            engine.handle_input("examine iron key"),
            vec![Event::Custom {
                name: "first-look".to_string()
            }]
        );
        // set the flag → not(flag) is false → stock fires
        engine.world_mut().set_flag("door-seen");
        assert_eq!(
            engine.handle_input("examine iron key"),
            vec![Event::Examined {
                target: Target::Object(ObjectId::new("iron-key")),
                target_name: "iron key".to_string(),
            }]
        );
    }

    #[test]
    fn multiple_flags_must_all_be_set() {
        let mut engine = engine_with(include_str!("fixtures/interactions/multiple_flags.yaml"));
        assert_eq!(
            engine.handle_input("take iron key"),
            vec![Event::Took {
                object_id: ObjectId::new("iron-key"),
                object: "iron key".to_string(),
            }]
        );
        // Neither set → stock
        assert_eq!(
            engine.handle_input("examine iron key"),
            vec![Event::Examined {
                target: Target::Object(ObjectId::new("iron-key")),
                target_name: "iron key".to_string(),
            }]
        );
        // Only one set → stock
        engine.world_mut().set_flag("step-one");
        assert_eq!(
            engine.handle_input("examine iron key"),
            vec![Event::Examined {
                target: Target::Object(ObjectId::new("iron-key")),
                target_name: "iron key".to_string(),
            }]
        );
        // Both set → custom
        engine.world_mut().set_flag("step-two");
        assert_eq!(
            engine.handle_input("examine iron key"),
            vec![Event::Custom {
                name: "both-steps-done".to_string()
            }]
        );
    }

    #[test]
    fn flag_combined_with_room_condition() {
        let mut engine = engine_with(include_str!("fixtures/interactions/flag_and_room.yaml"));
        assert_eq!(
            engine.handle_input("take iron key"),
            vec![Event::Took {
                object_id: ObjectId::new("iron-key"),
                object: "iron key".to_string(),
            }]
        );
        // flag set but still in cellar → stock
        engine.world_mut().set_flag("cellar-visited");
        assert_eq!(
            engine.handle_input("examine iron key"),
            vec![Event::Examined {
                target: Target::Object(ObjectId::new("iron-key")),
                target_name: "iron key".to_string(),
            }]
        );
        // flag set and in study → custom
        enter_study(&mut engine);
        assert_eq!(
            engine.handle_input("examine iron key"),
            vec![Event::Custom {
                name: "studied-key-in-study".to_string()
            }]
        );
    }

    #[test]
    fn set_and_clear_in_same_interaction_run_in_order() {
        let mut engine = engine_with(include_str!(
            "fixtures/interactions/set_and_clear_same.yaml"
        ));
        engine.world_mut().set_flag("phase-one");
        assert!(!engine.world().has_flag("phase-two"));
        assert_eq!(
            engine.handle_input("take iron key"),
            vec![Event::Took {
                object_id: ObjectId::new("iron-key"),
                object: "iron key".to_string(),
            }]
        );
        assert_eq!(
            engine.handle_input("examine iron key"),
            vec![
                Event::FlagSet {
                    flag: "phase-two".to_string()
                },
                Event::FlagCleared {
                    flag: "phase-one".to_string()
                },
                Event::Custom {
                    name: "transitioned".to_string()
                },
            ]
        );
        assert!(engine.world().has_flag("phase-two"));
        assert!(!engine.world().has_flag("phase-one"));
    }

    #[test]
    fn flag_effects_emit_events() {
        let mut engine = engine_with(include_str!("fixtures/interactions/set_flag_effect.yaml"));
        assert_eq!(
            engine.handle_input("take iron key"),
            vec![Event::Took {
                object_id: ObjectId::new("iron-key"),
                object: "iron key".to_string(),
            }]
        );
        enter_study(&mut engine);
        let events = engine.handle_input("use iron key on oak door");
        assert_eq!(
            events,
            vec![
                Event::FlagSet {
                    flag: "door-unlocked-flag".to_string()
                },
                Event::Custom {
                    name: "door-handled".to_string()
                },
            ]
        );
        assert!(engine.world().has_flag("door-unlocked-flag"));
    }

    #[test]
    fn flag_gated_interaction_overrides_stock_use() {
        let mut engine = engine_with(include_str!(
            "fixtures/interactions/flag_overrides_stock.yaml"
        ));
        assert_eq!(
            engine.handle_input("take iron key"),
            vec![Event::Took {
                object_id: ObjectId::new("iron-key"),
                object: "iron key".to_string(),
            }]
        );
        enter_study(&mut engine);
        // Flag not set → stock unlock fires
        assert_eq!(
            engine.handle_input("use iron key on oak door"),
            vec![Event::Custom {
                name: "no-flag-set".to_string()
            }]
        );
        // Reset the lock for the next attempt
        engine.world_mut().lock_exit(Direction::East);
        engine.world_mut().set_flag("override-stock");
        // Flag set → data interaction fires instead of stock
        assert_eq!(
            engine.handle_input("use iron key on oak door"),
            vec![Event::UnlockedExit {
                direction: Direction::East
            }]
        );
        assert!(!engine.world().is_exit_locked(Direction::East));
    }
}

// ---------------------------------------------------------------------------
// Query: interactions_for
// ---------------------------------------------------------------------------

mod interactions_for {
    use super::*;

    #[test]
    fn query_reports_interaction_when_flag_is_set() {
        let mut engine = engine_with(include_str!("fixtures/interactions/flag_query.yaml"));
        // Flag not set → query returns nothing
        assert!(
            engine
                .interactions_for(None, Some(Target::Object(ObjectId::new("iron-key"))))
                .is_empty()
        );
        // Set the flag → query returns the interaction
        engine.world_mut().set_flag("query-gate");
        assert_eq!(
            engine
                .interactions_for(None, Some(Target::Object(ObjectId::new("iron-key"))))
                .len(),
            1
        );
    }

    #[test]
    fn query_excludes_interaction_when_flag_is_cleared() {
        let mut engine = engine_with(include_str!("fixtures/interactions/flag_query.yaml"));
        engine.world_mut().set_flag("query-gate");
        assert_eq!(
            engine
                .interactions_for(None, Some(Target::Object(ObjectId::new("iron-key"))))
                .len(),
            1
        );
        engine.world_mut().clear_flag("query-gate");
        assert!(
            engine
                .interactions_for(
                    Some(ObjectId::new("iron-key")),
                    Some(Target::Object(ObjectId::new("oak-door")))
                )
                .is_empty()
        );
    }
}

// ---------------------------------------------------------------------------
// Initial flags
// ---------------------------------------------------------------------------

mod initial_flags {
    use super::*;

    #[test]
    fn initial_flags_are_set_on_construction() {
        let data = world_data_with_initial_flags(vec!["game-started", "intro-seen"]);
        let engine = GameEngine::get(&data);
        assert!(engine.world().has_flag("game-started"));
        assert!(engine.world().has_flag("intro-seen"));
    }

    #[test]
    fn no_initial_flags_means_empty() {
        let data = world_data_with_initial_flags(vec![]);
        let engine = GameEngine::get(&data);
        assert!(!engine.world().has_flag("anything"));
    }

    #[test]
    fn initial_flags_are_queryable_by_interactions() {
        let interactions = include_str!("fixtures/interactions/flag_condition.yaml");
        let interactions_yaml = format!("interactions:\n{interactions}");
        let mut data = WorldData::from_yaml(
            GLOBALS_YAML,
            ITEMS_YAML,
            ROOMS_YAML,
            &interactions_yaml,
            "{}",
        )
        .expect("parses");
        data.flags = vec!["quest-started".to_string()];
        let mut engine = GameEngine::get(&data);
        assert_eq!(
            engine.handle_input("take iron key"),
            vec![Event::Took {
                object_id: ObjectId::new("iron-key"),
                object: "iron key".to_string(),
            }]
        );
        // Flag is set from construction → interaction fires
        assert_eq!(
            engine.handle_input("examine iron key"),
            vec![Event::Custom {
                name: "quest-item-recognized".to_string()
            }]
        );
    }
}
