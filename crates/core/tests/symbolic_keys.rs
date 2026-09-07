//! Symbolic key-based authoring: schema and resolution spec (roadmap #1, part 2).
//!
//! Object and room identity is the stable `key: String` from the authored
//! YAML — the key **is** the `ObjectId`/`RoomId` (key-as-identity), and every
//! cross-reference (`visible_objects`, `hidden_objects`, door `to` /
//! `gated_by`, and every interaction field) is a string matched against those
//! keys. Numeric ids never appear in authored data.
//!
//! Unknown key and duplicate key are load errors (`WorldData::from_yaml`
//! returns `Err`).
//!
//! ## Fixtures (in `crates/core/data/`)
//!
//! * `data_interactions_items.yaml` — the 11-object world (keys `iron-key` …
//!   `oak-door`, plus the un-placed `rusty-nail`).
//! * `data_interactions_rooms.yaml` — the 3-room layout (Cellar → Corridor →
//!   Study) with key-based rooms.
//! * `interactions/` — authored interaction snippets (key-referencing).
//!
//! ## What this suite pins (the red-phase spec)
//!
//! 1. **End-to-end behavioural equivalence** — the keyed world behaves
//!    identically to the numeric `data_interactions` fixture: same navigation,
//!    same take/drop semantics, same data-interaction dispatch and query
//!    behaviour.
//! 2. **Every reference site resolves a key** — object lists, door `to` and
//!    `gated_by`, and all interaction `item`/`target`/`condition`/`effect`
//!    fields.
//! 3. **Integrity** — unknown key, duplicate key, and missing `key` fields
//!    are load errors.
//!
//! Run with: `cd crates/core && cargo test --test symbolic_keys`.

mod common;

use core::{Direction, Event, GameEngine, ObjectResolution, RoomId, Verb, WorldData};

const ITEMS_YAML: &str = include_str!("../data/data_interactions_items.yaml");
const ROOMS_YAML: &str = include_str!("../data/data_interactions_rooms.yaml");

/// The base keyed world (no interactions).
fn base_world() -> WorldData {
    WorldData::from_yaml(ITEMS_YAML, ROOMS_YAML, "{}").expect("keyed fixture parses")
}

/// The keyed world with an authored interaction snippet (same pattern as the
/// `data_interactions` suite).
fn world_with(interactions: &str) -> WorldData {
    let yaml = format!("interactions:\n{interactions}");
    WorldData::from_yaml(ITEMS_YAML, ROOMS_YAML, &yaml)
        .expect("keyed fixture with interactions parses")
}

fn engine_with(interactions: &str) -> GameEngine {
    GameEngine::get(&world_with(interactions))
}

/// Walks into The Study and takes the iron key. Uses key-resolved world state
/// for assertions (no hardcoded numeric ids).
fn iron_key_in_study(engine: &mut GameEngine) {
    engine.handle_input("take iron key");
    assert!(
        engine
            .world()
            .player_object_names()
            .contains(&"iron key".to_string())
    );
    assert_eq!(
        engine.handle_input("go north"),
        vec![Event::Went(Direction::North)]
    );
    assert_eq!(
        engine.handle_input("go east"),
        vec![Event::Went(Direction::East)]
    );
    assert!(
        engine
            .world()
            .room_object_names()
            .contains(&"oak door".to_string())
    );
}

// -----------------------------------------------------------------------
// Navigation / membership: proves keys resolve into objects and rooms
// -----------------------------------------------------------------------

mod keys {
    use super::*;

    #[test]
    fn visible_objects_key_resolves_and_room_count_is_correct() {
        let world = base_world();
        // Cellar visible: [iron-key, brass-key, rusty-lamp, cellar-stairs] → 4
        // Corridor: [corridor-stairs, study-door] → 2
        // Study:    [wooden-door, oak-door] → 2
        let engine = GameEngine::get(&world);
        let mut e = engine;
        assert_eq!(e.world().room_object_names().len(), 4);
        assert_eq!(
            e.handle_input("go north"),
            vec![Event::Went(Direction::North)]
        );
        assert_eq!(e.world().room_object_names().len(), 2);
        assert_eq!(
            e.handle_input("go east"),
            vec![Event::Went(Direction::East)]
        );
        assert_eq!(e.world().room_object_names().len(), 2);
    }

    #[test]
    fn hidden_objects_key_resolves_and_item_is_not_visible() {
        let engine = GameEngine::get(&base_world());
        // stale-bread is hidden in cellar: not in visible names, resolve returns NotFound.
        assert!(
            !engine
                .world()
                .room_object_names()
                .contains(&"stale bread".to_string())
        );
        assert_eq!(
            engine.world().resolve_target("stale bread"),
            ObjectResolution::NotFound
        );
    }

    #[test]
    fn door_to_key_resolves_via_navigation() {
        let mut engine = GameEngine::get(&base_world());
        // cellar → north → corridor (cellar-stairs door to: corridor)
        assert_eq!(
            engine.handle_input("go north"),
            vec![Event::Went(Direction::North)]
        );
        assert_eq!(engine.world().current_room_id(), RoomId::new("corridor"));
        // corridor → east → study (study-door door to: study)
        assert_eq!(
            engine.handle_input("go east"),
            vec![Event::Went(Direction::East)]
        );
        assert_eq!(engine.world().current_room_id(), RoomId::new("study"));
    }

    #[test]
    fn door_gated_by_key_resolves_via_stock_unlock() {
        let mut engine = GameEngine::get(&base_world());
        engine.handle_input("take iron key");
        assert_eq!(
            engine.handle_input("go north"),
            vec![Event::Went(Direction::North)]
        );
        assert_eq!(
            engine.handle_input("go east"),
            vec![Event::Went(Direction::East)]
        );
        // oak door: locked, gated_by iron-key. Stock fallback unlocks.
        assert_eq!(
            engine.handle_input("use iron key on oak door"),
            vec![Event::UnlockedExit {
                direction: Direction::East
            }]
        );
        assert!(!engine.world().is_exit_locked(Direction::East));
    }
}

// -----------------------------------------------------------------------
// Dispatch: data interactions authored with keys
// -----------------------------------------------------------------------

mod dispatch {
    use super::*;

    #[test]
    fn item_and_target_kind_keys_resolve() {
        let mut engine = engine_with(include_str!("../data/interactions/unlock_and_emit.yaml"));
        iron_key_in_study(&mut engine);
        assert!(engine.world().is_exit_locked(Direction::East));
        assert_eq!(
            engine.handle_input("use iron key on oak door"),
            vec![
                Event::UnlockedExit {
                    direction: Direction::East
                },
                Event::Custom {
                    name: "door-unlocked".to_string()
                },
            ]
        );
        assert!(!engine.world().is_exit_locked(Direction::East));
    }

    #[test]
    fn exact_object_target_key_resolves() {
        let mut engine = engine_with(include_str!(
            "../data/interactions/exact_object_target.yaml"
        ));
        iron_key_in_study(&mut engine);
        // wooden-door matches the authored target → Custom.
        assert_eq!(
            engine.handle_input("use iron key on wooden door"),
            vec![Event::Custom {
                name: "for-the-wooden-door".to_string()
            }]
        );
        // oak-door does not → stock UnlockedExit (gated_by resolves via key).
        assert_eq!(
            engine.handle_input("use iron key on oak door"),
            vec![Event::UnlockedExit {
                direction: Direction::East
            }]
        );
    }

    #[test]
    fn player_holds_condition_key_resolves() {
        let interactions = include_str!("../data/interactions/player_holds.yaml");
        // Without brass key: query is empty, stock unlock fires.
        let mut without_brass = engine_with(interactions);
        iron_key_in_study(&mut without_brass);
        let ObjectResolution::Found(iron_id) =
            without_brass.world().resolve_player_object("iron key")
        else {
            panic!("iron key should resolve");
        };
        let ObjectResolution::Found(oak_id) = without_brass.world().resolve_target("oak door")
        else {
            panic!("oak door should resolve");
        };
        assert!(
            without_brass
                .interactions_for(Some(iron_id), Some(oak_id))
                .is_empty()
        );
        assert_eq!(
            without_brass.handle_input("use iron key on oak door"),
            vec![Event::UnlockedExit {
                direction: Direction::East
            }]
        );

        // With brass key: query lists the interaction; dispatch runs it.
        let mut with_brass = engine_with(interactions);
        with_brass.handle_input("take iron key");
        with_brass.handle_input("take brass key");
        iron_key_in_study(&mut with_brass);
        let ObjectResolution::Found(iron_id) = with_brass.world().resolve_player_object("iron key")
        else {
            panic!("iron key should resolve");
        };
        let ObjectResolution::Found(oak_id) = with_brass.world().resolve_target("oak door") else {
            panic!("oak door should resolve");
        };
        assert_eq!(
            with_brass
                .interactions_for(Some(iron_id), Some(oak_id))
                .len(),
            1
        );
        assert_eq!(
            with_brass.handle_input("use iron key on oak door"),
            vec![Event::Custom {
                name: "brass-holder-key-worked".to_string()
            }]
        );
        // The effect owns the mutation: the lock stays.
        assert!(with_brass.world().is_exit_locked(Direction::East));
    }

    #[test]
    fn room_condition_key_resolves() {
        let mut in_cellar = engine_with(include_str!("../data/interactions/room_gate.yaml"));
        in_cellar.handle_input("take iron key");
        assert_eq!(
            in_cellar.handle_input("examine iron key"),
            vec![Event::Examined {
                object: "iron key".to_string(),
                object_id: match in_cellar.world().resolve_target("iron key") {
                    ObjectResolution::Found(id) => id,
                    _ => panic!("iron key resolves"),
                },
            }]
        );

        let mut in_study = engine_with(include_str!("../data/interactions/room_gate.yaml"));
        in_study.handle_input("take iron key");
        iron_key_in_study(&mut in_study);
        assert_eq!(
            in_study.handle_input("examine iron key"),
            vec![Event::Custom {
                name: "key-in-study".to_string()
            }]
        );
    }

    #[test]
    fn effect_object_keys_resolve() {
        let mut engine = engine_with(include_str!("../data/interactions/reveal_object.yaml"));
        iron_key_in_study(&mut engine);
        assert!(engine.world().is_exit_hidden(Direction::North));
        // The effect runs silently (no Examined, no Custom).
        assert_eq!(engine.handle_input("examine wooden door"), vec![]);
        assert!(!engine.world().is_exit_hidden(Direction::North));
        // secret-passage is now resolvable (prove the effect key resolved).
        assert!(matches!(
            engine.world().resolve_target("secret passage"),
            ObjectResolution::Found(_)
        ));
    }

    #[test]
    fn drop_effect_with_key_resolves() {
        let mut engine = engine_with(include_str!(
            "../data/interactions/drop_emits_then_drops.yaml"
        ));
        engine.handle_input("take iron key");
        engine.handle_input("take brass key");
        assert!(
            engine
                .world()
                .player_object_names()
                .contains(&"brass key".to_string())
        );
        // Resolve the id *before* dropping, so we can assert the `Dropped`
        // event reports the resolved key.
        let ObjectResolution::Found(brass_id) = engine.world().resolve_player_object("brass key")
        else {
            panic!("brass key resolves");
        };

        assert_eq!(
            engine.handle_input("drop brass key"),
            vec![
                Event::Custom {
                    name: "key-discarded".to_string()
                },
                Event::Dropped {
                    object: "brass key".to_string(),
                    object_id: brass_id,
                },
            ]
        );
        assert!(
            !engine
                .world()
                .player_object_names()
                .contains(&"brass key".to_string())
        );
        assert!(
            engine
                .world()
                .room_object_names()
                .contains(&"brass key".to_string())
        );
    }
}

// -----------------------------------------------------------------------
// Query: interactions_for resolves compiled key-based interactions
// -----------------------------------------------------------------------

mod query {
    use super::*;

    #[test]
    fn interactions_for_reports_compiled_item_key() {
        let mut engine = engine_with(include_str!("../data/interactions/unlock_and_emit.yaml"));
        // Carry the brass key too, so "wrong carried object" can be queried.
        engine.handle_input("take brass key");
        iron_key_in_study(&mut engine);
        let ObjectResolution::Found(iron_id) = engine.world().resolve_player_object("iron key")
        else {
            panic!("iron key resolves");
        };
        let ObjectResolution::Found(oak_id) = engine.world().resolve_target("oak door") else {
            panic!("oak door resolves");
        };
        let listed = engine.interactions_for(Some(iron_id.clone()), Some(oak_id.clone()));
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].verb(), Verb::Use);
        // The compiled interaction's item id matches the keyed resolution.
        assert_eq!(listed[0].item(), Some(iron_id));

        // Wrong carried object: no match.
        let ObjectResolution::Found(brass_id) = engine.world().resolve_player_object("brass key")
        else {
            panic!("brass key resolves");
        };
        assert!(
            engine
                .interactions_for(Some(brass_id), Some(oak_id))
                .is_empty()
        );
    }
}

// -----------------------------------------------------------------------
// Integrity: load errors for malformed keyed YAML
// -----------------------------------------------------------------------

mod integrity {
    use super::*;

    fn from_yaml(
        items: &str,
        rooms: &str,
        interactions: &str,
    ) -> Result<WorldData, Box<dyn std::error::Error>> {
        WorldData::from_yaml(items, rooms, interactions).map_err(Into::into)
    }

    // ---- unknown keys in references ----

    #[test]
    fn unknown_object_key_in_visible_objects_is_an_error() {
        let items = &ITEMS_YAML.to_string();
        let bad_rooms = r"rooms:
  - key: cellar
    name: The Cellar
    visible_objects: [iron-key, nonexistent-object]
    hidden_objects: []";
        let result = from_yaml(items, bad_rooms, "{}");
        assert!(result.is_err());
    }

    #[test]
    fn unknown_object_key_in_interaction_item_is_an_error() {
        let snippet = r"- verb: take
  item: nonexistent-object
  effect:
    - emit: nope";
        let result = from_yaml(ITEMS_YAML, ROOMS_YAML, &format!("interactions:\n{snippet}"));
        assert!(result.is_err());
    }

    #[test]
    fn unknown_room_key_in_door_to_is_an_error() {
        let items = r"objects:
  - key: bad-door
    primary_name: bad door
    kind: Scene
    door:
      direction: north
      to: nonexistent-room";
        let result = from_yaml(items, ROOMS_YAML, "{}");
        assert!(result.is_err());
    }

    #[test]
    fn unknown_room_key_in_condition_is_an_error() {
        let snippet = r"- verb: examine
  item: iron-key
  condition:
    - room: nonexistent-room";
        let result = from_yaml(ITEMS_YAML, ROOMS_YAML, &format!("interactions:\n{snippet}"));
        assert!(result.is_err());
    }

    // ---- duplicate keys ----

    #[test]
    fn duplicate_object_key_is_an_error() {
        let items = r"objects:
  - key: iron-key
    primary_name: iron key
    kind: Item
  - key: iron-key
    primary_name: duplicate iron key
    kind: Item";
        let result = from_yaml(items, ROOMS_YAML, "{}");
        assert!(result.is_err());
    }

    #[test]
    fn duplicate_room_key_is_an_error() {
        let rooms = r"rooms:
  - key: cellar
    name: The Cellar
    visible_objects: []
    hidden_objects: []
  - key: cellar
    name: Duplicate Cellar
    visible_objects: []
    hidden_objects: []";
        let result = from_yaml(ITEMS_YAML, rooms, "{}");
        assert!(result.is_err());
    }

    // ---- missing key field ----

    #[test]
    fn object_without_key_field_is_an_error() {
        let items = r"objects:
  - primary_name: no key item
    kind: Item";
        let result = from_yaml(items, ROOMS_YAML, "{}");
        assert!(result.is_err());
    }

    #[test]
    fn room_without_key_field_is_an_error() {
        let rooms = r"rooms:
  - name: No Key Room
    visible_objects: []
    hidden_objects: []";
        let result = from_yaml(ITEMS_YAML, rooms, "{}");
        assert!(result.is_err());
    }
}
