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

use core::{Direction, Event, GameEngine, Resolution, Rules, WorldState};
use common::{setup_engine, setup_engine_with_rules};

/// Overrides every hook to return a distinctive message so a test can tell at
/// a glance whether the override (and not the default) ran.
struct TestRules;

impl Rules for TestRules {
    fn on_look(&mut self, _world: &WorldState) -> Vec<Event> {
        vec![Event::Message("override: look".to_string())]
    }

    fn on_go(&mut self, _world: &mut WorldState, _direction: Direction) -> Vec<Event> {
        vec![Event::Message("override: go".to_string())]
    }

    fn on_take(
        &mut self,
        _world: &mut WorldState,
        _name: &str,
        _resolution: Resolution,
    ) -> Vec<Event> {
        vec![Event::Message("override: take".to_string())]
    }

    fn on_drop(
        &mut self,
        _world: &mut WorldState,
        _name: &str,
        _resolution: Resolution,
    ) -> Vec<Event> {
        vec![Event::Message("override: drop".to_string())]
    }

    fn on_examine(
        &mut self,
        _world: &WorldState,
        _name: &str,
        _resolution: Resolution,
    ) -> Vec<Event> {
        vec![Event::Message("override: examine".to_string())]
    }

    fn on_use(
        &mut self,
        _item: &str,
        _target: Option<&str>,
        _item_resolution: Resolution,
        _target_resolution: Resolution,
    ) -> Vec<Event> {
        vec![Event::Message("override: use".to_string())]
    }

    fn on_unknown(&mut self, _phrase: String) -> Vec<Event> {
        vec![Event::Message("override: unknown".to_string())]
    }
}

mod everything_overridden {
    use super::*;

    #[test]
    fn on_look_override_wins_over_looked() {
        let mut custom = setup_engine_with_rules(TestRules);
        let mut default = setup_engine();
        assert_eq!(
            vec![Event::Message("override: look".to_string())],
            custom.handle_input("look")
        );
        assert_eq!(vec![Event::Looked], default.handle_input("look"));
    }

    #[test]
    fn on_go_override_wins_over_went() {
        let mut custom = setup_engine_with_rules(TestRules);
        let mut default = setup_engine();
        assert_eq!(
            vec![Event::Message("override: go".to_string())],
            custom.handle_input("go north")
        );
        assert_eq!(
            vec![Event::Went(Direction::North)],
            default.handle_input("go north")
        );
    }

    #[test]
    fn on_take_override_wins_over_took() {
        let mut custom = setup_engine_with_rules(TestRules);
        let mut default = setup_engine();
        assert_eq!(
            vec![Event::Message("override: take".to_string())],
            custom.handle_input("take iron key")
        );
        assert_eq!(
            vec![Event::Took { item: "iron key".to_string() }],
            default.handle_input("take iron key")
        );
    }

    #[test]
    fn on_drop_override_wins_over_dropped() {
        let mut custom = setup_engine_with_rules(TestRules);
        assert_eq!(
            vec![Event::Message("override: drop".to_string())],
            custom.handle_input("drop iron key")
        );
    }

    #[test]
    fn on_examine_override_wins_over_default_message() {
        let mut custom = setup_engine_with_rules(TestRules);
        let mut default = setup_engine();
        assert_eq!(
            vec![Event::Message("override: examine".to_string())],
            custom.handle_input("examine iron key")
        );
        assert_eq!(
            vec![Event::Message("You examine the iron key.".to_string())],
            default.handle_input("examine iron key")
        );
    }

    #[test]
    fn on_use_override_wins_over_used() {
        let mut custom = setup_engine_with_rules(TestRules);
        let mut default = setup_engine();
        assert_eq!(
            vec![Event::Message("override: use".to_string())],
            custom.handle_input("use iron key")
        );
        default.handle_input("take iron key");
        assert_eq!(
            vec![Event::Used { item: "iron key".to_string(), target: None }],
            default.handle_input("use iron key")
        );
    }

    #[test]
    fn on_unknown_override_wins_over_default_message() {
        let mut custom = setup_engine_with_rules(TestRules);
        let mut default = setup_engine();
        assert_eq!(
            vec![Event::Message("override: unknown".to_string())],
            custom.handle_input("dance wildly")
        );
        assert_eq!(
            vec![Event::Message("I don't understand how to \"dance wildly\".".to_string())],
            default.handle_input("dance wildly")
        );
    }
}
