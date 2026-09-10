//! Shared helpers for integration tests under `tests/`.
//!
//! Individual test files include this via `mod common;`.

#![allow(dead_code)]

use core::{Direction, Event, GameEngine, RoomId, Rules, WorldData};

/// Concatenates YAML sections into the single document
/// [`WorldData::from_yaml`] parses. Each section already contributes its own
/// disjoint top-level key (`objects:`, `rooms:`, `interactions:`, ...), so
/// concatenation is a lossless merge.
pub(crate) fn merge_yaml(sections: &[&str]) -> String {
    sections.join("\n")
}

/// Loads the test-only multi-room world from `crates/core/tests/fixtures/`.
pub(crate) fn multi_room_world_data() -> WorldData {
    WorldData::from_yaml(include_str!("../fixtures/multi_room_world.yaml"))
        .expect("parse multi-room test world data")
}

/// Loads the original single-room world from `crates/core/tests/fixtures/`.
pub(crate) fn test_world_data() -> WorldData {
    WorldData::from_yaml(include_str!("../fixtures/single_room_world.yaml"))
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

// ---------------------------------------------------------------------------
// "Keyed" fixture: two rooms (`cellar` / `study`) connected by a `corridor`,
// used by the authored-interactions, flags, NPC and trigger suites. Shared
// here so each suite doesn't redeclare its own copy of the same loader
// functions.
// ---------------------------------------------------------------------------

pub(crate) const KEYED_WORLD_YAML: &str = include_str!("../fixtures/keyed_world.yaml");

/// The base keyed world: no interactions, no npcs, no flags, no triggers.
pub(crate) fn base_world() -> WorldData {
    WorldData::from_yaml(KEYED_WORLD_YAML).expect("keyed fixture parses")
}

/// The base keyed world with an initial flag set.
pub(crate) fn world_data_with_initial_flags(flags: Vec<&str>) -> WorldData {
    let mut data = base_world();
    data.flags = flags.into_iter().map(String::from).collect();
    data
}

/// The keyed world with an authored interaction snippet. `interactions` is a
/// standalone YAML sequence (as stored under `tests/fixtures/interactions/`),
/// wrapped under an `interactions:` key to become its own section.
pub(crate) fn world_with_interactions(interactions: &str) -> WorldData {
    let interactions_yaml = format!("interactions:\n{interactions}");
    WorldData::from_yaml(&merge_yaml(&[KEYED_WORLD_YAML, &interactions_yaml]))
        .expect("keyed fixture with authored interactions parses")
}

/// Opens the keyed world with an authored interaction snippet under the stock
/// rules.
pub(crate) fn engine_with_interactions(interactions: &str) -> GameEngine {
    GameEngine::get(&world_with_interactions(interactions))
}

/// The keyed world with an authored npcs snippet (already a full `npcs:`
/// section, as stored under `tests/fixtures/npcs_*.yaml`).
pub(crate) fn world_with_npcs(npcs: &str) -> WorldData {
    WorldData::from_yaml(&merge_yaml(&[KEYED_WORLD_YAML, npcs]))
        .expect("keyed fixture with npcs parses")
}

/// The keyed world with both an authored npcs snippet and an authored
/// interactions snippet.
pub(crate) fn world_with_npcs_and_interactions(npcs: &str, interactions: &str) -> WorldData {
    let interactions_yaml = format!("interactions:\n{interactions}");
    WorldData::from_yaml(&merge_yaml(&[KEYED_WORLD_YAML, npcs, &interactions_yaml]))
        .expect("keyed fixture with npcs and interactions parses")
}

/// Opens the keyed world with an authored npcs snippet under the stock rules.
pub(crate) fn engine_with_npcs(npcs: &str) -> GameEngine {
    GameEngine::get(&world_with_npcs(npcs))
}

/// The keyed world with an authored triggers snippet. `triggers` is a
/// standalone YAML sequence (as stored under `tests/fixtures/triggers/`),
/// wrapped under a `triggers:` key to become its own section, mirroring how
/// `world_with_interactions` wraps under an `interactions:` key.
pub(crate) fn world_with_triggers(triggers: &str) -> WorldData {
    let triggers_yaml = format!("triggers:\n{triggers}");
    WorldData::from_yaml(&merge_yaml(&[KEYED_WORLD_YAML, &triggers_yaml]))
        .expect("keyed fixture with triggers parses")
}

/// Opens the keyed world with an authored triggers snippet under the stock
/// rules.
pub(crate) fn engine_with_triggers(triggers: &str) -> GameEngine {
    GameEngine::get(&world_with_triggers(triggers))
}

/// Walks the keyed world's player from The Cellar into The Study via the
/// corridor.
pub(crate) fn enter_study(engine: &mut GameEngine) {
    assert_eq!(
        engine.handle_input("go north"),
        vec![Event::Went(Direction::North)]
    );
    assert_eq!(
        engine.handle_input("go east"),
        vec![Event::Went(Direction::East)]
    );
    assert_eq!(engine.world().current_room_id(), RoomId::new("study"));
}

/// Walks the keyed world's player from The Cellar into the corridor.
pub(crate) fn enter_corridor(engine: &mut GameEngine) {
    assert_eq!(
        engine.handle_input("go north"),
        vec![Event::Went(Direction::North)]
    );
    assert_eq!(engine.world().current_room_id(), RoomId::new("corridor"));
}
