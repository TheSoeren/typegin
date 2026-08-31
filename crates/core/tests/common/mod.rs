//! Shared helpers for integration tests under `tests/`.
//!
//! Individual test files include this via `mod common;`.

use core::{GameEngine, Rules, WorldData};

/// Loads the test-only multi-room world from `crates/core/data/`.
pub(crate) fn multi_room_world_data() -> WorldData {
    WorldData::from_toml(
        include_str!("../../data/items_multi_room.toml"),
        include_str!("../../data/rooms_multi_room.toml"),
    )
    .expect("parse multi-room test world data")
}

/// Loads the original single-room world from `crates/core/data/`.
pub(crate) fn test_world_data() -> WorldData {
    WorldData::from_toml(
        include_str!("../../data/items.toml"),
        include_str!("../../data/rooms.toml"),
    )
    .expect("parse single-room test world data")
}

/// Opens a multi-room engine with the given custom rules.
pub(crate) fn setup_engine_with_rules(rules: impl Rules + 'static) -> GameEngine {
    GameEngine::open_with_rules(":memory:", &multi_room_world_data(), rules)
        .expect("create engine with rules")
}

/// Opens a multi-room engine using the default `BasicRules`.
pub(crate) fn setup_engine() -> GameEngine {
    GameEngine::open(":memory:", &multi_room_world_data()).expect("create engine with BasicRules")
}

/// Opens a single-room engine using the default `BasicRules`.
pub(crate) fn single_room_engine() -> GameEngine {
    GameEngine::open(":memory:", &test_world_data())
        .expect("create single-room engine with BasicRules")
}
