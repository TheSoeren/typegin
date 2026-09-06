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
//!     same condition evaluation the dispatcher uses).
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
