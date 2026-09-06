//! Verifies that a custom `Rules` implementation actually overrides the trait
//! defaults, and that the defaults are used when a hook is not overridden.
//!
//! Every hook has a default implementation in the `Rules` trait (the same
//! behaviour `BasicRules` exposes). A bespoke type can override any subset.
//! These tests drive the same input through both `BasicRules` and a custom
//! `TestRules` that overrides every hook with a distinctive result, proving the
//! override wins over the default.
//!
//! Run with: cd crates/core && cargo test --test rules_override

mod common;

use common::{setup_engine, setup_engine_with_rules};
use core::{
    Direction, Event, GameEngine, Interaction, ObjectId, Rules, TargetFilter, Verb,
    world::{WorldState, object},
};

/// Overrides every hook to return a distinctive marker event so a test can
/// tell at a glance whether the override (and not the default) ran.
struct TestRules;

impl TestRules {
    fn override_event(label: &str) -> Vec<Event> {
        vec![Event::UnknownEvent {
            name: format!("override: {label}"),
        }]
    }
}

impl Rules for TestRules {
    fn on_look(&mut self, _world: &mut WorldState) -> Vec<Event> {
        Self::override_event("look")
    }

    fn on_go(&mut self, _world: &mut WorldState, _direction: Direction) -> Vec<Event> {
        Self::override_event("go")
    }

    fn on_take(
        &mut self,
        _world: &mut WorldState,
        _name: &str,
        _resolution: object::ObjectResolution,
    ) -> Vec<Event> {
        Self::override_event("take")
    }

    fn on_drop(
        &mut self,
        _world: &mut WorldState,
        _name: &str,
        _resolution: object::ObjectResolution,
    ) -> Vec<Event> {
        Self::override_event("drop")
    }

    fn on_examine(
        &mut self,
        _world: &mut WorldState,
        _name: &str,
        _resolution: object::ObjectResolution,
    ) -> Vec<Event> {
        Self::override_event("examine")
    }

    fn on_use(
        &mut self,
        _world: &mut WorldState,
        _item: &str,
        _target: Option<&str>,
        _item_resolution: object::ObjectResolution,
        _target_resolution: object::ObjectResolution,
    ) -> Vec<Event> {
        Self::override_event("use")
    }

    fn on_unknown(&mut self, _world: &mut WorldState, _phrase: String) -> Vec<Event> {
        Self::override_event("unknown")
    }
}

fn marker(label: &str) -> Vec<Event> {
    TestRules::override_event(label)
}

mod everything_overridden {
    use super::*;

    #[test]
    fn on_look_override_wins_over_looked() {
        let mut custom = setup_engine_with_rules(TestRules);
        let mut default = setup_engine();
        assert_eq!(marker("look"), custom.handle_input("look"));
        assert_eq!(vec![Event::Looked], default.handle_input("look"));
    }

    #[test]
    fn on_go_override_wins_over_went() {
        let mut custom = setup_engine_with_rules(TestRules);
        let mut default = setup_engine();
        assert_eq!(marker("go"), custom.handle_input("go north"));
        assert_eq!(
            vec![Event::Went(Direction::North)],
            default.handle_input("go north")
        );
    }

    #[test]
    fn on_take_override_wins_over_took() {
        let mut custom = setup_engine_with_rules(TestRules);
        let mut default = setup_engine();
        assert_eq!(marker("take"), custom.handle_input("take iron key"));
        assert_eq!(
            vec![Event::Took {
                object_id: ObjectId::new(2),
                object: "iron key".to_string()
            }],
            default.handle_input("take iron key")
        );
    }

    #[test]
    fn on_drop_override_wins_over_dropped() {
        let mut custom = setup_engine_with_rules(TestRules);
        assert_eq!(marker("drop"), custom.handle_input("drop iron key"));
    }

    #[test]
    fn on_examine_override_wins_over_examined() {
        let mut custom = setup_engine_with_rules(TestRules);
        let mut default = setup_engine();
        assert_eq!(marker("examine"), custom.handle_input("examine iron key"));
        assert_eq!(
            vec![Event::Examined {
                object_id: ObjectId::new(2),
                object: "iron key".to_string(),
            }],
            default.handle_input("examine iron key")
        );
    }

    #[test]
    fn on_use_override_wins_over_used() {
        let mut custom = setup_engine_with_rules(TestRules);
        let mut default = setup_engine();
        assert_eq!(marker("use"), custom.handle_input("use iron key"));
        default.handle_input("take iron key");
        assert_eq!(
            vec![Event::UsedTargetNeeded {
                object_id: ObjectId::new(2),
                object: "iron key".to_string()
            }],
            default.handle_input("use iron key")
        );
    }

    #[test]
    fn on_unknown_override_wins_over_unknown_event() {
        let mut custom = setup_engine_with_rules(TestRules);
        let mut default = setup_engine();
        assert_eq!(marker("unknown"), custom.handle_input("dance wildly"));
        assert_eq!(
            vec![Event::UnknownEvent {
                name: "dance wildly".to_string()
            }],
            default.handle_input("dance wildly")
        );
    }

    #[test]
    fn interaction_without_condition_runs_instead_of_default() {
        // A consumer `Rules` type that only *adds* authored interactions and
        // keeps the stock hooks runs them before the default `UsedTargetNeeded`
        // fallback, because `BasicRules::on_use` consults `interactions()`.
        struct Authored(Vec<Interaction>);
        impl Rules for Authored {
            fn interactions(&self) -> &[Interaction] {
                &self.0
            }
        }

        let interactions = vec![Interaction::build(
            Verb::Use,
            None,
            TargetFilter::Any,
            None,
            Box::new(|_world, _context| {
                vec![Event::UnknownEvent {
                    name: "authored".to_string(),
                }]
            }),
        )];
        let mut engine =
            GameEngine::get_with_rules(&common::multi_room_world_data(), Authored(interactions));
        engine.handle_input("take iron key");
        assert_eq!(
            engine.handle_input("use iron key"),
            vec![Event::UnknownEvent {
                name: "authored".to_string()
            }]
        );
    }
}
