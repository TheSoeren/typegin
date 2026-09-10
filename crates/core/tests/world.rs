//! Tests for `WorldState`: entity resolution and item movement.
//!
//! Run with: cd crates/core && cargo test --test world

mod common;

use common::single_room_engine;
use core::{GameEngine, ObjectId, ObjectResolution, Outcome, RoomId, world::WorldState};

/// A fresh engine over the single-room world, whose room `cellar` holds the
/// sword, iron key, locked chest and brass key visibly, and the stale bread
/// hidden.
fn engine() -> GameEngine {
    single_room_engine()
}

/// Assert the current room holds (or not) an item by id.
fn room_has_item(world: &WorldState, id: &ObjectId) -> bool {
    matches!(world.get_object_from_room(id), ObjectResolution::Found(_))
}

/// Assert the player holds (or not) an item by id.
fn player_has_item(world: &WorldState, id: &ObjectId) -> bool {
    matches!(world.get_object_from_player(id), ObjectResolution::Found(_))
}

mod resolution {
    use core::{Target, world::object::TargetResolution};

    use super::*;

    fn resolves(name: &str) -> TargetResolution {
        engine().world().resolve_target(name)
    }

    #[test]
    fn exact_full_name() {
        assert_eq!(
            TargetResolution::Found(Target::Object(ObjectId::new("glowing-sword"))),
            resolves("glowing mysterious sword")
        );
    }

    #[test]
    fn partial_alias_match() {
        assert_eq!(
            TargetResolution::Found(Target::Object(ObjectId::new("glowing-sword"))),
            resolves("glowing sword")
        );
    }

    #[test]
    fn alias_match() {
        assert_eq!(
            TargetResolution::Found(Target::Object(ObjectId::new("iron-key"))),
            resolves("iron key")
        );
    }

    #[test]
    fn ambiguous_key() {
        assert_eq!(
            TargetResolution::Ambiguous {
                ids: vec![
                    Target::Object(ObjectId::new("iron-key")),
                    Target::Object(ObjectId::new("brass-key"))
                ],
                alias: "key".to_string()
            },
            resolves("key")
        );
    }

    #[test]
    fn not_found() {
        assert_eq!(TargetResolution::NotFound, resolves("health potion"));
    }
}

mod worlds_inventory {
    use super::*;

    #[test]
    fn seed_populates_room_items_and_empty_inventory() {
        let engine = engine();
        assert!(room_has_item(
            engine.world(),
            &ObjectId::new("glowing-sword")
        ));
        assert!(room_has_item(engine.world(), &ObjectId::new("iron-key")));
        assert!(!room_has_item(
            engine.world(),
            &ObjectId::new("stale-bread")
        ));
        assert!(engine.world().player_object_names().is_empty());
    }

    #[test]
    fn take_item_success() {
        let mut engine = engine();
        let result = engine
            .world_mut()
            .player_take_object(&ObjectId::new("iron-key"));
        assert_eq!(result, Outcome::Success);
        assert!(!room_has_item(engine.world(), &ObjectId::new("iron-key")));
        assert!(player_has_item(engine.world(), &ObjectId::new("iron-key")));
    }

    #[test]
    fn take_item_not_in_room_fails() {
        let mut engine = engine();
        // Stale bread is hidden in this world, so taking it from the room is
        // not possible.
        let result = engine
            .world_mut()
            .player_take_object(&ObjectId::new("stale-bread"));
        assert_eq!(result, Outcome::Fail);
        assert!(!room_has_item(
            engine.world(),
            &ObjectId::new("stale-bread")
        ));
        assert!(!player_has_item(
            engine.world(),
            &ObjectId::new("stale-bread")
        ));
    }

    #[test]
    fn drop_item_returns_to_room() {
        let mut engine = engine();
        engine
            .world_mut()
            .player_take_object(&ObjectId::new("iron-key"));
        let result = engine
            .world_mut()
            .player_drop_object(&ObjectId::new("iron-key"));
        assert_eq!(result, Outcome::Success);
        assert!(!player_has_item(engine.world(), &ObjectId::new("iron-key")));
        assert!(room_has_item(engine.world(), &ObjectId::new("iron-key")));
    }

    #[test]
    fn drop_item_not_held_fails() {
        let mut engine = engine();
        let result = engine
            .world_mut()
            .player_drop_object(&ObjectId::new("iron-key"));
        assert_eq!(result, Outcome::Fail);
        assert!(room_has_item(engine.world(), &ObjectId::new("iron-key")));
    }

    #[test]
    fn move_to_unknown_room_fails() {
        let mut engine = engine();
        // The single-room world has no other room.
        assert_eq!(
            engine.world_mut().move_to_room(RoomId::new("nonexistent")),
            Outcome::Fail
        );
        assert_eq!(engine.world().current_room_id(), RoomId::new("cellar"));
    }
}
