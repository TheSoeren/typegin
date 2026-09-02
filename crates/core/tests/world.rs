//! Tests for `WorldState`: entity resolution and item movement.
//!
//! Run with: cd crates/core && cargo test --test world

mod common;

use common::single_room_engine;
use core::{
    DropResult, GameEngine, ItemId, ItemResolution, MoveResult, RoomId, TakeResult,
    world::WorldState,
};

/// A fresh engine over the single-room world, whose room 1 holds items
/// 1, 2, 3, 4 visibly and item 5 hidden.
fn engine() -> GameEngine {
    single_room_engine()
}

/// Assert the current room holds (or not) an item by id.
fn room_has_item(world: &WorldState, id: i32) -> bool {
    matches!(
        world.get_item_from_room(ItemId::new(id)),
        ItemResolution::Found(_)
    )
}

/// Assert the player holds (or not) an item by id.
fn player_has_item(world: &WorldState, id: i32) -> bool {
    matches!(
        world.get_item_from_player(ItemId::new(id)),
        ItemResolution::Found(_)
    )
}

mod resolution {
    use super::*;

    fn resolves(name: &str) -> ItemResolution {
        engine().world().resolve_any_item(name)
    }

    #[test]
    fn exact_full_name() {
        assert_eq!(
            ItemResolution::Found(ItemId::new(1)),
            resolves("glowing mysterious sword")
        );
    }

    #[test]
    fn partial_alias_match() {
        assert_eq!(
            ItemResolution::Found(ItemId::new(1)),
            resolves("glowing sword")
        );
    }

    #[test]
    fn alias_match() {
        assert_eq!(ItemResolution::Found(ItemId::new(2)), resolves("iron key"));
    }

    #[test]
    fn ambiguous_key() {
        assert_eq!(
            ItemResolution::Ambiguous(vec![ItemId::new(2), ItemId::new(4)]),
            resolves("key")
        );
    }

    #[test]
    fn not_found() {
        assert_eq!(ItemResolution::NotFound, resolves("health potion"));
    }
}

mod worlds_inventory {
    use super::*;

    #[test]
    fn seed_populates_room_items_and_empty_inventory() {
        let engine = engine();
        assert!(room_has_item(engine.world(), 1));
        assert!(room_has_item(engine.world(), 2));
        assert!(!room_has_item(engine.world(), 5)); // hidden item is not visible
        assert!(engine.world().player_item_names().is_empty());
    }

    #[test]
    fn take_item_success() {
        let mut engine = engine();
        let result = engine.world_mut().player_take_item(ItemId::new(2));
        assert_eq!(result, TakeResult::Success);
        assert!(!room_has_item(engine.world(), 2));
        assert!(player_has_item(engine.world(), 2));
    }

    #[test]
    fn take_item_not_in_room_fails() {
        let mut engine = engine();
        // Item 5 is hidden, so taking it from the room is not possible.
        let result = engine.world_mut().player_take_item(ItemId::new(5));
        assert_eq!(result, TakeResult::Fail);
        assert!(!room_has_item(engine.world(), 5));
        assert!(!player_has_item(engine.world(), 5));
    }

    #[test]
    fn drop_item_returns_to_room() {
        let mut engine = engine();
        engine.world_mut().player_take_item(ItemId::new(2));
        let result = engine.world_mut().player_drop_item(ItemId::new(2));
        assert_eq!(result, DropResult::Success);
        assert!(!player_has_item(engine.world(), 2));
        assert!(room_has_item(engine.world(), 2));
    }

    #[test]
    fn drop_item_not_held_fails() {
        let mut engine = engine();
        let result = engine.world_mut().player_drop_item(ItemId::new(2));
        assert_eq!(result, DropResult::Fail);
        assert!(room_has_item(engine.world(), 2));
    }

    #[test]
    fn move_to_unknown_room_fails() {
        let mut engine = engine();
        // The single-room world has no room 2.
        assert_eq!(
            engine.world_mut().move_to_room(RoomId::new(2)),
            MoveResult::Fail
        );
        assert_eq!(engine.world().current_room_id(), RoomId::new(1));
    }
}
