//! Tests for `WorldState`: entity resolution and item movement.
//!
//! Run with: cd crates/core && cargo test --test world

mod common;

use core::{GameEngine, Resolution};
use common::single_room_engine;

/// A fresh engine over the single-room world, whose room 1 holds items
/// 1, 2, 3, 4 visibly and item 5 hidden.
fn engine() -> GameEngine {
    single_room_engine()
}

mod resolution {
    use super::*;

    fn resolves(name: &str) -> Resolution {
        engine().world.resolve_any_item(name)
    }

    #[test]
    fn exact_full_name() {
        assert_eq!(Resolution::Found(1), resolves("glowing mysterious sword"));
    }

    #[test]
    fn partial_alias_match() {
        assert_eq!(Resolution::Found(1), resolves("glowing sword"));
    }

    #[test]
    fn alias_match() {
        assert_eq!(Resolution::Found(2), resolves("iron key"));
    }

    #[test]
    fn ambiguous_key() {
        assert_eq!(Resolution::Ambiguous(vec![2, 4]), resolves("key"));
    }

    #[test]
    fn not_found() {
        assert_eq!(Resolution::NotFound, resolves("health potion"));
    }
}

mod worlds_inventory {
    use super::*;

    #[test]
    fn seed_populates_room_items_and_empty_inventory() {
        let engine = engine();
        assert!(engine.world.room_has_item(1));
        assert!(engine.world.room_has_item(2));
        assert!(!engine.world.room_has_item(5)); // hidden item is not visible
        assert!(engine.world.inventory_item_names().is_empty());
    }

    #[test]
    fn take_item_success() {
        let mut engine = engine();
        let result = engine.world.move_item_to_inventory(2);
        assert!(result);
        assert!(!engine.world.room_has_item(2));
        assert!(engine.world.player_has_item(2));
    }

    #[test]
    fn take_item_already_in_inventory_fails() {
        let mut engine = engine();
        engine.world.move_item_to_inventory(2);
        let result = engine.world.move_item_to_inventory(2);
        assert!(!result);
    }

    #[test]
    fn drop_item_returns_to_room() {
        let mut engine = engine();
        engine.world.move_item_to_inventory(2);
        let result = engine.world.move_item_from_inventory(2);
        assert!(result);
        assert!(!engine.world.player_has_item(2));
        assert!(engine.world.room_has_item(2));
    }

    #[test]
    fn drop_item_not_held_fails() {
        let mut engine = engine();
        assert!(!engine.world.move_item_from_inventory(2));
        assert!(engine.world.room_has_item(2));
    }
}
