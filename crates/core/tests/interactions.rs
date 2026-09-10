mod common;

use common::setup_engine;
use core::object_data::ObjectKind;
use core::{
    ActionContext, Direction, Event, GameEngine, Interaction, ObjectId, Rules, Target,
    TargetFilter, Verb, WorldState,
};

/// Takes the iron key (cellar), walks north (corridor) and east (study) so the
/// player stands in the Dusty Study facing the locked oak door.
fn setup_engine_in_study_with_iron_key() -> GameEngine {
    let mut engine = setup_engine();
    engine.handle_input("take iron key");
    engine.handle_input("go north");
    engine.handle_input("go east");
    engine
}

mod unlock {
    use super::*;

    #[test]
    fn unlocked_door_becomes_traversable() {
        let mut engine = setup_engine_in_study_with_iron_key();
        engine.handle_input("use iron key on oak door");
        assert_eq!(
            engine.handle_input("go east"),
            vec![Event::Went(Direction::East)]
        );
        assert_eq!(
            engine.world().current_room_id(),
            core::RoomId::new("corridor")
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
                object_id: ObjectId::new("iron-key"),
                object: "iron key".to_string(),
                target: "hidden vault".to_string(),
            }]
        );
        assert!(engine.world().is_exit_locked(Direction::South));
    }
}

mod resolve_target {
    use core::world::object::TargetResolution;

    use super::*;

    #[test]
    fn doors_resolve_by_their_first_class_name() {
        let engine = setup_engine_in_study_with_iron_key();
        assert_eq!(
            engine.world().resolve_target("oak door"),
            TargetResolution::Found(Target::Object(ObjectId::new("oak-door")))
        );
        assert_eq!(
            engine.world().resolve_target("wooden door"),
            TargetResolution::Found(Target::Object(ObjectId::new("wooden-door")))
        );
    }

    #[test]
    fn items_and_doors_share_one_noun_path() {
        let engine = setup_engine_in_study_with_iron_key();
        assert_eq!(
            engine.world().resolve_target("iron key"),
            TargetResolution::Found(Target::Object(ObjectId::new("iron-key")))
        );
        assert_eq!(
            engine.world().resolve_target("oak door"),
            TargetResolution::Found(Target::Object(ObjectId::new("oak-door")))
        );
    }

    #[test]
    fn hidden_doors_do_not_resolve() {
        let engine = setup_engine_in_study_with_iron_key();
        assert_eq!(
            engine.world().resolve_target("secret passage"),
            TargetResolution::NotFound
        );
        assert_eq!(
            engine.world().resolve_target("hidden vault"),
            TargetResolution::NotFound
        );
    }

    #[test]
    fn ambiguous_key_in_scope_reports_both_candidates() {
        let mut engine = setup_engine();
        engine.handle_input("take iron key");
        engine.handle_input("take brass key");
        assert_eq!(
            engine.world().resolve_target("key"),
            TargetResolution::Ambiguous {
                ids: vec![
                    Target::Object(ObjectId::new("iron-key")),
                    Target::Object(ObjectId::new("brass-key"))
                ],
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
        assert_eq!(engine.world().current_room_id(), core::RoomId::new("study"));
    }

    #[test]
    fn carryable_items_are_the_default_kind() {
        let engine = setup_engine_in_study_with_iron_key();
        assert_eq!(
            engine.world().object_kind(&ObjectId::new("iron-key")),
            Some(ObjectKind::Item)
        );
        assert!(engine.world().player_holds(&ObjectId::new("iron-key")));
    }

    #[test]
    fn doors_are_scene_objects_with_door_data() {
        let engine = setup_engine_in_study_with_iron_key();
        assert_eq!(
            engine.world().object_kind(&ObjectId::new("oak-door")),
            Some(ObjectKind::Scene)
        );
        assert!(engine.world().object_is_door(&ObjectId::new("oak-door")));
        assert!(!engine.world().object_is_scene(&ObjectId::new("iron-key")));
        assert_eq!(
            engine.world().exit_direction_of(&ObjectId::new("oak-door")),
            Some(Direction::East)
        );
    }

    #[test]
    fn examine_works_on_exits_like_any_scene_object() {
        let mut engine = setup_engine();
        assert_eq!(
            engine.handle_input("examine stairs"),
            vec![Event::Examined {
                target: Target::Object(ObjectId::new("cellar-stairs")),
                target_name: "stairs".to_string(),
            }]
        );
    }
}

mod target_in_scope {
    use super::*;

    #[test]
    fn carried_item_and_visible_door_are_in_scope() {
        let engine = setup_engine_in_study_with_iron_key();
        assert!(engine.world().target_in_scope(&ObjectId::new("iron-key")));
        assert!(engine.world().target_in_scope(&ObjectId::new("oak-door")));
    }

    #[test]
    fn hidden_door_is_not_in_scope() {
        let engine = setup_engine_in_study_with_iron_key();
        assert!(
            !engine
                .world()
                .target_in_scope(&ObjectId::new("hidden-vault"))
        );
    }
}

mod interactions_for {
    use super::*;

    #[test]
    fn query_lists_matching_interactions_and_conditions_gate_both_paths() {
        let mut engine = GameEngine::get(&common::multi_room_world_data());
        engine.handle_input("take iron key");
        engine.handle_input("go north");
        engine.handle_input("go east");

        // While locked: both the item-specific `use` interaction and the
        // item-agnostic `examine` interaction (no `item` field, so it matches
        // any carried item) are listed for this target.
        assert_eq!(
            engine
                .interactions_for(
                    Some(ObjectId::new("iron-key")),
                    Some(Target::Object(ObjectId::new("oak-door")))
                )
                .len(),
            2
        );
        assert_eq!(
            engine.handle_input("use iron key on oak door"),
            vec![
                Event::UnlockedExit {
                    direction: Direction::East
                },
                Event::Custom {
                    name: "unlock-authored".to_string(),
                }
            ]
        );
        assert!(!engine.world().is_exit_locked(Direction::East));

        // Now unlocked: the `use` interaction's condition no longer holds, so
        // the query drops it — but the unconditional `examine` interaction
        // still matches.
        let after_unlock = engine.interactions_for(
            Some(ObjectId::new("iron-key")),
            Some(Target::Object(ObjectId::new("oak-door"))),
        );
        assert_eq!(after_unlock.len(), 1);
        assert_eq!(after_unlock[0].verb(), Verb::Examine);
    }

    #[test]
    fn query_only_matches_the_authored_item() {
        let mut engine = GameEngine::get(&common::multi_room_world_data());
        engine.handle_input("take iron key");
        engine.handle_input("go north");
        engine.handle_input("go east");

        let with_iron_key = engine.interactions_for(
            Some(ObjectId::new("iron-key")),
            Some(Target::Object(ObjectId::new("oak-door"))),
        );
        assert_eq!(with_iron_key.len(), 2);
        assert!(with_iron_key.iter().any(|i| i.verb() == Verb::Use));

        // Iron key in hand but querying with a different carried object: the
        // item-specific `use` interaction no longer matches, but the
        // item-agnostic `examine` interaction (no `item` field) still does.
        let with_brass_key = engine.interactions_for(
            Some(ObjectId::new("brass-key")),
            Some(Target::Object(ObjectId::new("oak-door"))),
        );
        assert_eq!(with_brass_key.len(), 1);
        assert_eq!(with_brass_key[0].verb(), Verb::Examine);
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
    use core::world::object::TargetResolution;

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
        let mut engine = GameEngine::get(&common::multi_room_world_data());
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
            TargetResolution::Found(Target::Object(ObjectId::new("secret-passage")))
        );
        assert!(!engine.world().is_exit_hidden(Direction::North));
    }

    #[test]
    fn stock_examine_runs_for_objects_with_no_interaction() {
        let interactions = vec![Interaction::build(
            Verb::Examine,
            Some(ObjectId::new("oak-door")),
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
                target: Target::Object(ObjectId::new("wooden-door")),
                target_name: "wooden door".to_string(),
            }]
        );
    }

    #[test]
    fn drop_interaction_replaces_stock_drop_and_owns_the_mutation() {
        let mut engine = GameEngine::get(&common::multi_room_world_data());
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
        assert!(engine.world().player_holds(&ObjectId::new("old-map")));
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
                object_id: ObjectId::new("iron-key"),
                object: "iron key".to_string(),
            }]
        );
    }

    #[test]
    fn unresolvable_names_never_reach_interactions() {
        let mut engine = GameEngine::get(&common::multi_room_world_data());
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
        assert!(!engine.world().player_holds(&ObjectId::new("glowing-sword")));
    }

    #[test]
    fn interactions_for_reports_every_verb() {
        let mut engine = GameEngine::get(&common::multi_room_world_data());
        engine.handle_input("go north");
        engine.handle_input("go east");

        // Each item returns its matching interaction — verb-independent.
        let examine =
            engine.interactions_for(None, Some(Target::Object(ObjectId::new("oak-door"))));
        assert_eq!(examine.len(), 1);
        assert_eq!(examine[0].verb(), Verb::Examine);

        let take = engine.interactions_for(Some(ObjectId::new("glowing-sword")), None);
        assert_eq!(take.len(), 1);
        assert_eq!(take[0].verb(), Verb::Take);

        let drop = engine.interactions_for(Some(ObjectId::new("old-map")), None);
        assert_eq!(drop.len(), 1);
        assert_eq!(drop[0].verb(), Verb::Drop);

        // This target also carries the item-agnostic `examine` interaction
        // from the fixture, so both are reported for any carried item.
        let use_it = engine.interactions_for(
            Some(ObjectId::new("iron-key")),
            Some(Target::Object(ObjectId::new("oak-door"))),
        );
        assert_eq!(use_it.len(), 2);
        assert_eq!(use_it[0].verb(), Verb::Use);
        assert_eq!(use_it[1].verb(), Verb::Examine);
    }
}
