//! Shared helpers for integration tests under `tests/`.
//!
//! Individual test files include this via `mod common;`.

#![allow(dead_code)]

use core::{GameEngine, Rules, WorldData};

/// Loads the test-only multi-room world from `crates/core/data/`.
pub(crate) fn multi_room_world_data() -> WorldData {
    WorldData::from_yaml(
        include_str!("../../data/items_multi_room.yaml"),
        include_str!("../../data/rooms_multi_room.yaml"),
    )
    .expect("parse multi-room test world data")
}

/// Loads the original single-room world from `crates/core/data/`.
pub(crate) fn test_world_data() -> WorldData {
    WorldData::from_yaml(
        include_str!("../../data/items.yaml"),
        include_str!("../../data/rooms.yaml"),
    )
    .expect("parse single-room test world data")
}

/// Opens a multi-room engine with the given custom rules.
pub(crate) fn setup_engine_with_rules(rules: impl Rules + 'static) -> GameEngine {
    GameEngine::get_with_rules(&multi_room_world_data(), rules)
}

/// Opens a multi-room engine using the default `BasicRules`.
pub(crate) fn setup_engine() -> GameEngine {
    GameEngine::get(&multi_room_world_data())
}

/// Opens a single-room engine using the default `BasicRules`.
pub(crate) fn single_room_engine() -> GameEngine {
    GameEngine::get(&test_world_data())
}
