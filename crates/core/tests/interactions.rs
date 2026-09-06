//! The Visionaire-style interaction core: items and doors share one object
//! model, scene objects are not portable, and authored interactions plug into
//! the stock `BasicRules::on_use` before its fallback spine runs.
//!
//! Covers:
//!   * the scene-object vs inventory-object distinction (exactly two kinds:
//!     `Item` / `Scene`) — doors are scene objects, so they can be examined but
//!     never taken,
//!   * resolving a locked door by name and unlocking it with its `gated_by`
//!     object (and only that object),
//!   * hidden doors being invisible to resolution (they are not targets),
//!   * unified item/object resolution through `WorldState::resolve_target`,
//!   * the `GameEngine::interactions_for` point-and-click query (with the
//!     same condition evaluation the dispatcher uses),
//!   * the same authored-interaction surface extending past `Use` to
//!     `Examine`, `Take` and `Drop`, and the query reporting all verbs.
//!
//! Run with: cd crates/core && cargo test --test interactions

mod common;

use common::setup_engine;
use core::{
    ActionContext, Direction, Event, GameEngine, Interaction, ObjectId, ObjectKind,
    ObjectResolution, Rules, TargetFilter, Verb, WorldState,
};

/// Takes the iron key (room 1), walks north (room 2) and east (room 3) so the
/// player stands in the Dusty Study facing the locked oak door.
fn setup_engine_in_study_with_iron_key() -> GameEngine {
    let mut engine = setup_engine();
    engine.handle_input("take iron key");
    engine.handle_input("go north");
    engine.handle_input("go east");
    engine
}

/// Takes the brass key (room 1) and walks to the study.
fn setup_engine_in_study_with_brass_key() -> GameEngine {
    let mut engine = setup_engine();
    engine.handle_input("take brass key");
    engine.handle_input("go north");
    engine.handle_input("go east");
    engine
}

mod unlock {
    use super::*;

    #[test]
    fn use_gating_object_unlocks_the_door() {
        let mut engine = setup_engine_in_study_with_iron_key();
        assert!(engine.world().is_exit_locked(Direction::East));
        assert_eq!(
            engine.handle_input("use iron key on oak door"),
            vec![Event::UnlockedExit {
                direction: Direction::East
            }]
        );
        assert!(!engine.world().is_exit_locked(Direction::East));
        assert!(!engine.world().is_exit_hidden(Direction::East));
    }

    #[test]
    fn unlocked_door_becomes_traversable() {
        let mut engine = setup_engine_in_study_with_iron_key();
        engine.handle_input("use iron key on oak door");
        assert_eq!(
            engine.handle_input("go east"),
            vec![Event::Went(Direction::East)]
        );
        assert_eq!(engine.world().current_room_id(), core::RoomId::new(2));
    }

    #[test]
    fn wrong_key_cannot_unlock() {
        let mut engine = setup_engine_in_study_with_brass_key();
        assert_eq!(
            engine.handle_input("use brass key on oak door"),
            vec![Event::CannotUse {
                item: "brass key".to_string(),
                target: "oak door".to_string(),
            }]
        );
        assert!(engine.world().is_exit_locked(Direction::East));
    }

    #[test]
    fn use_on_unlocked_door_is_a_generic_refusal() {
        let mut engine = setup_engine_in_study_with_iron_key();
        assert_eq!(
            engine.handle_input("use iron key on wooden door"),
            vec![Event::CannotUse {
                item: "iron key".to_string(),
                target: "wooden door".to_string(),
            }]
        );
    }

    #[test]
    fn reusing_the_key_after_unlock_is_a_generic_refusal() {
        let mut engine = setup_engine_in_study_with_iron_key();
        engine.handle_input("use iron key on oak door");
        assert_eq!(
            engine.handle_input("use iron key on oak door"),
            vec![Event::CannotUse {
                item: "iron key".to_string(),
                target: "oak door".to_string(),
            }]
        );
    }

    #[test]
    fn hidden_door_is_not_usable_as_target() {
        let mut engine = setup_engine_in_study_with_iron_key();
        // The hidden vault (south) is hidden, so it never resolves; the fallback
        // reports the player trying to use the key on something unknown.
        assert_eq!(
            engine.handle_input("use iron key on hidden vault"),
            vec![Event::UsedTargetNotFound {
                object_id: ObjectId::new(2),
                object: "iron key".to_string(),
                target: "hidden vault".to_string(),
            }]
        );
        assert!(engine.world().is_exit_locked(Direction::South));
    }
}

mod resolve_target {
    use super::*;

    #[test]
    fn doors_resolve_by_their_first_class_name() {
        let engine = setup_engine_in_study_with_iron_key();
        assert_eq!(
            engine.world().resolve_target("oak door"),
            ObjectResolution::Found(ObjectId::new(14))
        );
        assert_eq!(
            engine.world().resolve_target("wooden door"),
            ObjectResolution::Found(ObjectId::new(11))
        );
    }

    #[test]
    fn items_and_doors_share_one_noun_path() {
        let engine = setup_engine_in_study_with_iron_key();
        assert_eq!(
            engine.world().resolve_target("iron key"),
            ObjectResolution::Found(ObjectId::new(2))
        );
        assert_eq!(
            engine.world().resolve_target("oak door"),
            ObjectResolution::Found(ObjectId::new(14))
        );
    }

    #[test]
    fn hidden_doors_do_not_resolve() {
        let engine = setup_engine_in_study_with_iron_key();
        assert_eq!(
            engine.world().resolve_target("secret passage"),
            ObjectResolution::NotFound
        );
        assert_eq!(
            engine.world().resolve_target("hidden vault"),
            ObjectResolution::NotFound
        );
    }

    #[test]
    fn ambiguous_key_in_scope_reports_both_candidates() {
        let mut engine = setup_engine();
        engine.handle_input("take iron key");
        engine.handle_input("take brass key");
        assert_eq!(
            engine.world().resolve_target("key"),
            ObjectResolution::Ambiguous {
                ids: vec![ObjectId::new(2), ObjectId::new(4)],
                alias: "key".to_string(),
            }
        );
    }
}

mod scene_vs_inventory {
    use super::*;

    #[test]
    fn scene_objects_are_not_portable() {
        let mut engine = setup_engine_in_study_with_iron_key();
        assert_eq!(
            engine.handle_input("take oak door"),
            vec![Event::CantTake {
                object: "oak door".to_string()
            }]
        );
        assert_eq!(
            engine.handle_input("take wooden door"),
            vec![Event::CantTake {
                object: "wooden door".to_string()
            }]
        );
        // Still in the room, still a working door.
        assert!(
            !engine
                .world()
                .player_object_names()
                .contains(&"oak door".to_string())
        );
        assert!(!engine.world().is_exit_locked(Direction::West));
        assert_eq!(engine.world().current_room_id(), core::RoomId::new(3));
    }

    #[test]
    fn carryable_items_are_the_default_kind() {
        let engine = setup_engine_in_study_with_iron_key();
        assert_eq!(
            engine.world().object_kind(ObjectId::new(2)),
            Some(ObjectKind::Item)
        );
        assert!(engine.world().player_holds(ObjectId::new(2)));
    }

    #[test]
    fn doors_are_scene_objects_with_door_data() {
        let engine = setup_engine_in_study_with_iron_key();
        assert_eq!(
            engine.world().object_kind(ObjectId::new(14)),
            Some(ObjectKind::Scene)
        );
        assert!(engine.world().object_is_door(ObjectId::new(14)));
        assert!(!engine.world().object_is_scene(ObjectId::new(2)));
        assert_eq!(
            engine.world().exit_direction_of(ObjectId::new(14)),
            Some(Direction::East)
        );
    }

    #[test]
    fn examine_works_on_doors_like_any_scene_object() {
        let mut engine = setup_engine_in_study_with_iron_key();
        assert_eq!(
            engine.handle_input("examine oak door"),
            vec![Event::Examined {
                object_id: ObjectId::new(14),
                object: "oak door".to_string(),
            }]
        );
    }
}

mod target_in_scope {
    use super::*;

    #[test]
    fn carried_item_and_visible_door_are_in_scope() {
        let engine = setup_engine_in_study_with_iron_key();
        assert!(engine.world().target_in_scope(ObjectId::new(2)));
        assert!(engine.world().target_in_scope(ObjectId::new(14)));
    }

    #[test]
    fn hidden_door_is_not_in_scope() {
        let engine = setup_engine_in_study_with_iron_key();
        assert!(!engine.world().target_in_scope(ObjectId::new(13)));
    }
}

mod interactions_for {
    use super::*;

    #[test]
    fn stock_rules_report_no_authored_interactions() {
        let engine = setup_engine_in_study_with_iron_key();
        assert!(
            engine
                .interactions_for(Some(ObjectId::new(2)), Some(ObjectId::new(14)))
                .is_empty()
        );
    }

    #[test]
    fn query_lists_matching_interactions_and_conditions_gate_both_paths() {
        // Authored: "use iron key on a door, but only while the east door is
        // still locked", with a *custom* unlock (emits a Custom beat instead of
        // the stock UnlockedExit). Keeps the default on_use, so the same
        // condition that gates the point-and-click query also gates the
        // dispatch.
        struct AuthoredRules(Vec<Interaction>);
        impl Rules for AuthoredRules {
            fn interactions(&self) -> &[Interaction] {
                &self.0
            }
        }

        let interactions = vec![Interaction::build(
            Verb::Use,
            Some(ObjectId::new(2)),
            TargetFilter::Door,
            Some(Box::new(|world: &WorldState, _context: &ActionContext| {
                world.is_exit_locked(Direction::East)
            })),
            Box::new(|world: &mut WorldState, _context: &ActionContext| {
                world.unlock_exit(Direction::East);
                vec![Event::Custom {
                    name: "unlock-authored".to_string(),
                }]
            }),
        )];
        let mut engine = GameEngine::get_with_rules(
            &common::multi_room_world_data(),
            AuthoredRules(interactions),
        );
        engine.handle_input("take iron key");
        engine.handle_input("go north");
        engine.handle_input("go east");

        // While locked: the interaction is listed, and the dispatcher runs it
        // (custom unlock event) instead of the stock UnlockedExit.
        assert_eq!(
            engine
                .interactions_for(Some(ObjectId::new(2)), Some(ObjectId::new(14)))
                .len(),
            1
        );
        assert_eq!(
            engine.handle_input("use iron key on oak door"),
            vec![Event::Custom {
                name: "unlock-authored".to_string(),
            }]
        );
        assert!(!engine.world().is_exit_locked(Direction::East));

        // Now unlocked: the condition no longer holds, so the query drops it
        // and the dispatcher falls through to the CannotUse spine.
        assert!(
            engine
                .interactions_for(Some(ObjectId::new(2)), Some(ObjectId::new(14)))
                .is_empty()
        );
        assert_eq!(
            engine.handle_input("use iron key on oak door"),
            vec![Event::CannotUse {
                item: "iron key".to_string(),
                target: "oak door".to_string(),
            }]
        );
    }

    #[test]
    fn query_only_matches_the_authored_item() {
        struct SystemRules(Vec<Interaction>);
        impl Rules for SystemRules {
            fn interactions(&self) -> &[Interaction] {
                &self.0
            }
        }

        let interactions = vec![Interaction::build(
            Verb::Use,
            Some(ObjectId::new(2)),
            TargetFilter::Door,
            None,
            Box::new(|_world: &mut WorldState, _context: &ActionContext| Vec::new()),
        )];
        let mut engine =
            GameEngine::get_with_rules(&common::multi_room_world_data(), SystemRules(interactions));
        engine.handle_input("take iron key");
        engine.handle_input("go north");
        engine.handle_input("go east");

        assert_eq!(
            engine
                .interactions_for(Some(ObjectId::new(2)), Some(ObjectId::new(14)))
                .len(),
            1
        );
        // Iron key in hand but querying with a different carried object.
        assert!(
            engine
                .interactions_for(Some(ObjectId::new(4)), Some(ObjectId::new(14)))
                .is_empty()
        );
    }
}

/// The interaction surface extends past `Use`: the same "first matching
/// interaction for the resolved object wins, else stock" gate applies to
/// `Examine`, `Take` and `Drop`, and `interactions_for` reports every verb.
///
/// Contract (the spec this suite pins down):
/// 1. The default `on_examine`/`on_take`/`on_drop` run the *first* matching
///    authored interaction before their stock spine — but only when the
///    object resolved (`Found`); bad names never reach interactions.
/// 2. A matching interaction fully replaces stock behaviour: the effect owns
///    the world mutation (via `&mut WorldState`) and the returned events.
/// 3. `ActionContext` carries the resolved object as `item` and `None` as
///    `target` for these verbs, so `TargetFilter` must be `Any` to match.
/// 4. `GameEngine::interactions_for` reports matching interactions for any
///    verb (no `Use`-only filter).
mod non_use_verbs {
    use super::*;

    struct AuthoredInteractions(Vec<Interaction>);
    impl Rules for AuthoredInteractions {
        fn interactions(&self) -> &[Interaction] {
            &self.0
        }
    }

    fn engine_with(interactions: Vec<Interaction>) -> GameEngine {
        GameEngine::get_with_rules(
            &common::multi_room_world_data(),
            AuthoredInteractions(interactions),
        )
    }

    #[test]
    fn examine_interaction_runs_first_and_can_mutate_the_world() {
        // "Examining the oak door in the study reveals the secret passage."
        let interactions = vec![Interaction::build(
            Verb::Examine,
            Some(ObjectId::new(14)),
            TargetFilter::Any,
            None,
            Box::new(|world: &mut WorldState, _context: &ActionContext| {
                let _ = world.reveal_object(ObjectId::new(12));
                vec![Event::Custom {
                    name: "passage-found".to_string(),
                }]
            }),
        )];
        let mut engine = engine_with(interactions);
        engine.handle_input("go north");
        engine.handle_input("go east");

        // The authored beat replaces the stock `Examined`...
        assert_eq!(
            engine.handle_input("examine oak door"),
            vec![Event::Custom {
                name: "passage-found".to_string(),
            }]
        );
        // ...and its world mutation took effect: the hidden door is now visible.
        assert_eq!(
            engine.world().resolve_target("secret passage"),
            ObjectResolution::Found(ObjectId::new(12))
        );
        assert!(!engine.world().is_exit_hidden(Direction::North));
    }

    #[test]
    fn stock_examine_runs_for_objects_with_no_interaction() {
        let interactions = vec![Interaction::build(
            Verb::Examine,
            Some(ObjectId::new(14)),
            TargetFilter::Any,
            None,
            Box::new(|_world: &mut WorldState, _context: &ActionContext| Vec::new()),
        )];
        let mut engine = engine_with(interactions);
        engine.handle_input("go north");
        engine.handle_input("go east");

        assert_eq!(
            engine.handle_input("examine wooden door"),
            vec![Event::Examined {
                object_id: ObjectId::new(11),
                object: "wooden door".to_string(),
            }]
        );
    }

    #[test]
    fn take_interaction_gated_by_condition_overrides_stock_take() {
        // "The sword can only be lifted while holding the lamp."
        let interactions = vec![Interaction::build(
            Verb::Take,
            Some(ObjectId::new(1)),
            TargetFilter::Any,
            Some(Box::new(|world: &WorldState, _context: &ActionContext| {
                world.player_holds(ObjectId::new(6))
            })),
            Box::new(|world: &mut WorldState, _context: &ActionContext| {
                let _ = world.player_take_object(ObjectId::new(1));
                vec![Event::Custom {
                    name: "sword-taken-under-light".to_string(),
                }]
            }),
        )];
        let mut engine = engine_with(interactions);
        // The condition gates the query too: before holding the lamp the
        // interaction is not listed.
        assert!(
            engine
                .interactions_for(Some(ObjectId::new(1)), None)
                .is_empty()
        );

        // Condition not met: stock take, no authored beat.
        assert_eq!(
            engine.handle_input("take sword"),
            vec![Event::Took {
                object_id: ObjectId::new(1),
                object: "sword".to_string(),
            }]
        );

        // Drop the sword, fetch the lamp, take again: now the interaction fires.
        engine.handle_input("drop sword");
        engine.handle_input("go north");
        engine.handle_input("take rusty lamp");
        engine.handle_input("go south");
        // Lamp in hand: the condition holds, so the query now lists it.
        assert_eq!(
            engine.interactions_for(Some(ObjectId::new(1)), None).len(),
            1
        );
        assert_eq!(
            engine.handle_input("take sword"),
            vec![Event::Custom {
                name: "sword-taken-under-light".to_string(),
            }]
        );
        assert!(engine.world().player_holds(ObjectId::new(1)));
    }

    #[test]
    fn drop_interaction_replaces_stock_drop_and_owns_the_mutation() {
        let interactions = vec![Interaction::build(
            Verb::Drop,
            Some(ObjectId::new(7)),
            TargetFilter::Any,
            None,
            Box::new(|_world: &mut WorldState, _context: &ActionContext| {
                vec![Event::Custom {
                    name: "map-returned".to_string(),
                }]
            }),
        )];
        let mut engine = engine_with(interactions);

        // The interaction fires instead of `Dropped`, and the effect did not
        // drop the map — full replacement, the map stays carried.
        engine.handle_input("take iron key");
        engine.handle_input("go north");
        engine.handle_input("take old map");
        assert_eq!(
            engine.handle_input("drop map"),
            vec![Event::Custom {
                name: "map-returned".to_string(),
            }]
        );
        assert!(engine.world().player_holds(ObjectId::new(7)));
        assert!(
            !engine
                .world()
                .room_object_names()
                .contains(&"old map".to_string())
        );
        assert!(
            engine
                .world()
                .player_object_names()
                .contains(&"old map".to_string())
        );

        // Objects without an authored beat keep the stock drop.
        assert_eq!(
            engine.handle_input("drop iron key"),
            vec![Event::Dropped {
                object_id: ObjectId::new(2),
                object: "iron key".to_string(),
            }]
        );
    }

    #[test]
    fn unresolvable_names_never_reach_interactions() {
        // A matching-item-less Take interaction would intercept *every* take —
        // but only ones that resolved.
        let interactions = vec![Interaction::build(
            Verb::Take,
            None,
            TargetFilter::Any,
            None,
            Box::new(|_world: &mut WorldState, _context: &ActionContext| {
                vec![Event::Custom {
                    name: "intercepted".to_string(),
                }]
            }),
        )];
        let mut engine = engine_with(interactions);

        assert_eq!(
            engine.handle_input("take nonexistent"),
            vec![Event::TookObjectNotFound {
                object: "nonexistent".to_string(),
            }]
        );
        // A resolved object *is* intercepted, and the stock take does not
        // happen (the sword stays in the room).
        assert_eq!(
            engine.handle_input("take sword"),
            vec![Event::Custom {
                name: "intercepted".to_string(),
            }]
        );
        assert!(!engine.world().player_holds(ObjectId::new(1)));
    }

    #[test]
    fn interactions_for_reports_every_verb() {
        let interactions = vec![
            Interaction::build(
                Verb::Examine,
                Some(ObjectId::new(14)),
                TargetFilter::Any,
                None,
                Box::new(|_world: &mut WorldState, _context: &ActionContext| Vec::new()),
            ),
            Interaction::build(
                Verb::Take,
                Some(ObjectId::new(1)),
                TargetFilter::Any,
                None,
                Box::new(|_world: &mut WorldState, _context: &ActionContext| Vec::new()),
            ),
            Interaction::build(
                Verb::Drop,
                Some(ObjectId::new(7)),
                TargetFilter::Any,
                None,
                Box::new(|_world: &mut WorldState, _context: &ActionContext| Vec::new()),
            ),
            Interaction::build(
                Verb::Use,
                Some(ObjectId::new(2)),
                TargetFilter::Door,
                None,
                Box::new(|_world: &mut WorldState, _context: &ActionContext| Vec::new()),
            ),
        ];
        // Navigate to room 3 so the oak door is in scope for TargetFilter::Door.
        let mut engine = engine_with(interactions);
        engine.handle_input("go north");
        engine.handle_input("go east");

        // Each item returns its matching interaction — verb-independent.
        let examine = engine.interactions_for(Some(ObjectId::new(14)), None);
        assert_eq!(examine.len(), 1);
        assert_eq!(examine[0].verb(), Verb::Examine);

        let take = engine.interactions_for(Some(ObjectId::new(1)), None);
        assert_eq!(take.len(), 1);
        assert_eq!(take[0].verb(), Verb::Take);

        let drop = engine.interactions_for(Some(ObjectId::new(7)), None);
        assert_eq!(drop.len(), 1);
        assert_eq!(drop[0].verb(), Verb::Drop);

        let use_it = engine.interactions_for(Some(ObjectId::new(2)), Some(ObjectId::new(14)));
        assert_eq!(use_it.len(), 1);
        assert_eq!(use_it[0].verb(), Verb::Use);
    }
}
