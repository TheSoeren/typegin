//! Spec for "combine two carried items" (AGENTS.md engine gap #4, second
//! half): scoping an authored interaction to a *carried* target, distinct
//! from a *scene* (room) target, on the existing `Action::Use { item, target
//! }` path. No new `Verb`, `Action`, or `Event` — "combine key with string"
//! is just `use key on string` where `string` happens to resolve to a
//! carried object instead of a room object, a path
//! `WorldState::resolve_target` already supports (it searches room objects
//! *and* inventory) but no suite has exercised end to end until now.
//!
//! ## Schema additions
//!
//! `TargetKind` (shared, like `Verb`, between the authored schema and the
//! compiled runtime filter — see `crates/core/src/interaction/target.rs`)
//! gains a `Carried` variant alongside the existing `Scene`:
//!
//! ```yaml
//! interactions:
//!   - verb: use
//!     item: brass-key
//!     target:
//!       kind: carried     # any *carried* object, not one fixed key
//!     effect:
//!       - emit: key-touched-something-carried
//! ```
//!
//! `target: kind: carried` matches *any* currently-carried object, the way
//! `target: kind: scene` already matches any room object — it authors a
//! reaction that should fire regardless of *which* carried object the item
//! was used on, not a fixed two-item recipe. A specific recipe ("brass key +
//! rusty lamp -> rusty nail") is instead authored the way it already works
//! today: a fixed `target: object: <id>` naming the second item by key,
//! assembling the result from the existing `Discard`/`Grant` effects (no new
//! effect kind) — this suite pins that already-working path down too, since
//! it was previously untested.
//!
//! ## Dispatch contract
//!
//! 1. `use <item> on <target>` resolves `target` via
//!    `WorldState::resolve_target`, which searches the current room *and*
//!    the player's inventory — unchanged by this feature.
//! 2. An interaction scoped `target: kind: carried`
//!    (`TargetFilter::Kind(TargetKind::Carried)`) matches only when the
//!    resolved target is `Target::Object(id)` and `world.player_holds(id)`;
//!    it never matches an NPC target or a room object, symmetric with
//!    `target: kind: scene` (`TargetFilter::Kind(TargetKind::Scene)`) never
//!    matching a carried object.
//! 3. When no authored interaction matches (target not carried, or nothing
//!    authored at all), dispatch falls through to the stock `on_use`
//!    fallback exactly as for any other `use` — a carried target is not a
//!    distinct code path, only a distinct *filter* an author opts into.
//! 4. Item/target resolution failures continue to report through the
//!    existing `UsedObjectNotFound` / `UsedObjectAmbiguous` /
//!    `UsedTargetNotFound` / `UsedTargetAmbiguous` events — no new event
//!    vocabulary for combine.
//!
//! Run with: `cargo test --test combine`.

mod common;

use common::{engine_with_interactions as engine_with, world_with_interactions as world_with};
use core::{DataTarget, Event, GameEngine, ObjectId, Target, TargetKind, Verb};

/// Takes the brass key and rusty lamp, leaving the player in The Cellar.
fn carry_brass_key_and_rusty_lamp(engine: &mut GameEngine) {
    assert_eq!(
        engine.handle_input("take brass key"),
        vec![Event::Took {
            object_id: ObjectId::new("brass-key"),
            object: "brass key".to_string(),
        }]
    );
    assert_eq!(
        engine.handle_input("take rusty lamp"),
        vec![Event::Took {
            object_id: ObjectId::new("rusty-lamp"),
            object: "rusty lamp".to_string(),
        }]
    );
}

// ---------------------------------------------------------------------------
// Schema: `target: kind: carried`
// ---------------------------------------------------------------------------

mod parse {
    use super::*;

    #[test]
    fn target_kind_carried_parses_as_target_kind_carried() {
        let world = world_with(
            r"- verb: use
  item: brass-key
  target:
    kind: carried
  effect:
    - emit: combined",
        );
        assert_eq!(
            world.interactions[0].target,
            Some(DataTarget::Kind {
                kind: TargetKind::Carried
            })
        );
    }

    #[test]
    fn target_kind_carried_and_scene_are_distinct_variants() {
        let world = world_with(
            r"- verb: use
  item: brass-key
  target:
    kind: carried
  effect:
    - emit: combined
- verb: use
  item: brass-key
  target:
    kind: scene
  effect:
    - emit: scene-beat",
        );
        assert_eq!(
            world.interactions[0].target,
            Some(DataTarget::Kind {
                kind: TargetKind::Carried
            })
        );
        assert_eq!(
            world.interactions[1].target,
            Some(DataTarget::Kind {
                kind: TargetKind::Scene
            })
        );
        assert_ne!(world.interactions[0].target, world.interactions[1].target);
    }
}

// ---------------------------------------------------------------------------
// A specific two-item recipe (already-working `target: object: <id>` path,
// previously untested with a carried target)
// ---------------------------------------------------------------------------

mod combine_dispatch {
    use super::*;

    const SPECIFIC_RECIPE_YAML: &str = r"- verb: use
  item: brass-key
  target:
    object: rusty-lamp
  effect:
    - discard: brass-key
    - discard: rusty-lamp
    - grant: rusty-nail
    - emit: combined";

    #[test]
    fn combine_two_carried_items_via_specific_target_succeeds() {
        let mut engine = engine_with(SPECIFIC_RECIPE_YAML);
        carry_brass_key_and_rusty_lamp(&mut engine);

        assert_eq!(
            engine.handle_input("use brass key on rusty lamp"),
            vec![
                Event::Discarded {
                    object_id: ObjectId::new("brass-key"),
                    object: "brass key".to_string(),
                },
                Event::Discarded {
                    object_id: ObjectId::new("rusty-lamp"),
                    object: "rusty lamp".to_string(),
                },
                Event::Granted {
                    object_id: ObjectId::new("rusty-nail"),
                    object: "rusty nail".to_string(),
                },
                Event::Custom {
                    name: "combined".to_string(),
                },
            ]
        );
        assert!(engine.world().player_holds(&ObjectId::new("rusty-nail")));
        assert!(!engine.world().player_holds(&ObjectId::new("brass-key")));
        assert!(!engine.world().player_holds(&ObjectId::new("rusty-lamp")));
    }
}

// ---------------------------------------------------------------------------
// `target: kind: carried` (`TargetFilter::Kind(TargetKind::Carried)`) scoping
// ---------------------------------------------------------------------------

mod carried_kind_scope {
    use super::*;

    const KIND_CARRIED_YAML: &str = r"- verb: use
  item: brass-key
  target:
    kind: carried
  effect:
    - emit: combined";

    const KIND_SCENE_YAML: &str = r"- verb: use
  item: brass-key
  target:
    kind: scene
  effect:
    - emit: scene-beat";

    const BOTH_SCOPES_YAML: &str = r"- verb: use
  item: brass-key
  target:
    kind: carried
  effect:
    - emit: combine-beat
- verb: use
  item: brass-key
  target:
    kind: scene
  effect:
    - emit: scene-beat";

    #[test]
    fn kind_carried_target_fires_for_any_carried_object_not_just_one() {
        let mut engine = engine_with(KIND_CARRIED_YAML);
        assert_eq!(
            engine.handle_input("take iron key"),
            vec![Event::Took {
                object_id: ObjectId::new("iron-key"),
                object: "iron key".to_string(),
            }]
        );
        carry_brass_key_and_rusty_lamp(&mut engine);

        assert_eq!(
            engine.handle_input("use brass key on iron key"),
            vec![Event::Custom {
                name: "combined".to_string(),
            }]
        );
        assert_eq!(
            engine.handle_input("use brass key on rusty lamp"),
            vec![Event::Custom {
                name: "combined".to_string(),
            }]
        );
    }

    #[test]
    fn kind_carried_target_rejects_a_room_object_target() {
        let mut engine = engine_with(KIND_CARRIED_YAML);
        assert_eq!(
            engine.handle_input("take brass key"),
            vec![Event::Took {
                object_id: ObjectId::new("brass-key"),
                object: "brass key".to_string(),
            }]
        );
        // `cellar-stairs` is a Scene object left in the room, not carried, so
        // `target: kind: carried` never matches it.
        assert_eq!(
            engine.handle_input("use brass key on cellar stairs"),
            vec![Event::Used {
                object_id: ObjectId::new("brass-key"),
                object: "brass key".to_string(),
                target_id: Some(Target::Object(ObjectId::new("cellar-stairs"))),
                target: Some("cellar stairs".to_string()),
            }]
        );
    }

    #[test]
    fn kind_scene_target_rejects_a_carried_target() {
        let mut engine = engine_with(KIND_SCENE_YAML);
        carry_brass_key_and_rusty_lamp(&mut engine);

        // The vice versa: `target: kind: scene` never matches a carried object.
        assert_eq!(
            engine.handle_input("use brass key on rusty lamp"),
            vec![Event::Used {
                object_id: ObjectId::new("brass-key"),
                object: "brass key".to_string(),
                target_id: Some(Target::Object(ObjectId::new("rusty-lamp"))),
                target: Some("rusty lamp".to_string()),
            }]
        );
    }

    #[test]
    fn carried_and_scene_scoped_interactions_coexist_without_cross_firing() {
        let mut engine = engine_with(BOTH_SCOPES_YAML);
        carry_brass_key_and_rusty_lamp(&mut engine);

        assert_eq!(
            engine.handle_input("use brass key on rusty lamp"),
            vec![Event::Custom {
                name: "combine-beat".to_string(),
            }]
        );
        assert_eq!(
            engine.handle_input("use brass key on cellar stairs"),
            vec![Event::Custom {
                name: "scene-beat".to_string(),
            }]
        );
    }

    #[test]
    fn interactions_for_reports_the_carried_scoped_interaction_only_for_a_carried_target() {
        let mut engine = engine_with(KIND_CARRIED_YAML);
        carry_brass_key_and_rusty_lamp(&mut engine);

        let for_carried = engine.interactions_for(
            Some(ObjectId::new("brass-key")),
            Some(Target::Object(ObjectId::new("rusty-lamp"))),
        );
        assert_eq!(for_carried.len(), 1);
        assert_eq!(for_carried[0].verb(), Verb::Use);

        let for_scene = engine.interactions_for(
            Some(ObjectId::new("brass-key")),
            Some(Target::Object(ObjectId::new("cellar-stairs"))),
        );
        assert!(for_scene.is_empty());
    }
}

// ---------------------------------------------------------------------------
// Not-found / ambiguous edge cases (existing `Used*` event vocabulary,
// unchanged by combine)
// ---------------------------------------------------------------------------

mod not_found_and_ambiguous {
    use super::*;

    const KIND_CARRIED_YAML: &str = r"- verb: use
  item: brass-key
  target:
    kind: carried
  effect:
    - emit: combined";

    #[test]
    fn combine_item_not_carried_is_used_object_not_found() {
        let mut engine = engine_with(KIND_CARRIED_YAML);
        assert_eq!(
            engine.handle_input("take rusty lamp"),
            vec![Event::Took {
                object_id: ObjectId::new("rusty-lamp"),
                object: "rusty lamp".to_string(),
            }]
        );
        assert_eq!(
            engine.handle_input("use brass key on rusty lamp"),
            vec![Event::UsedObjectNotFound {
                object: "brass key".to_string(),
            }]
        );
    }

    #[test]
    fn combine_target_not_in_scope_is_used_target_not_found() {
        let mut engine = engine_with(KIND_CARRIED_YAML);
        assert_eq!(
            engine.handle_input("take brass key"),
            vec![Event::Took {
                object_id: ObjectId::new("brass-key"),
                object: "brass key".to_string(),
            }]
        );
        assert_eq!(
            engine.handle_input("use brass key on phantom"),
            vec![Event::UsedTargetNotFound {
                object_id: ObjectId::new("brass-key"),
                object: "brass key".to_string(),
                target: "phantom".to_string(),
            }]
        );
    }

    #[test]
    fn combine_item_alias_ambiguous_between_two_carried_items() {
        let mut engine = engine_with(KIND_CARRIED_YAML);
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
        assert_eq!(
            engine.handle_input("take rusty lamp"),
            vec![Event::Took {
                object_id: ObjectId::new("rusty-lamp"),
                object: "rusty lamp".to_string(),
            }]
        );
        // Both keys share the alias "key", so the item is ambiguous.
        assert_eq!(
            engine.handle_input("use key on rusty lamp"),
            vec![Event::UsedObjectAmbiguous {
                object_ids: vec![ObjectId::new("iron-key"), ObjectId::new("brass-key")],
                object: "key".to_string(),
            }]
        );
    }

    #[test]
    fn combine_target_alias_ambiguous_between_two_carried_items() {
        let mut engine = engine_with(KIND_CARRIED_YAML);
        assert_eq!(
            engine.handle_input("take rusty lamp"),
            vec![Event::Took {
                object_id: ObjectId::new("rusty-lamp"),
                object: "rusty lamp".to_string(),
            }]
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
        // Both keys share the alias "key", so the *target* is ambiguous —
        // this has no precedent test elsewhere since it only arises for a
        // carried target.
        assert_eq!(
            engine.handle_input("use rusty lamp on key"),
            vec![Event::UsedTargetAmbiguous {
                object_id: ObjectId::new("rusty-lamp"),
                object: "rusty lamp".to_string(),
                target_ids: vec![
                    Target::Object(ObjectId::new("iron-key")),
                    Target::Object(ObjectId::new("brass-key")),
                ],
                target: "key".to_string(),
            }]
        );
    }
}
