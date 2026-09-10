mod common;

use core::{Direction, Event, GameEngine, ObjectResolution, RoomId, Verb, WorldData};

use common::{base_world, engine_with_interactions as engine_with, merge_yaml};

/// The `objects:` and `rooms:` sections of the keyed world, split apart so
/// `mod integrity` below can corrupt just one while keeping the other
/// canonical. Kept in sync with `fixtures/keyed_world.yaml` by hand — this is
/// the one file that needs the two sections independently addressable.
const ITEMS_YAML: &str = r"objects:
  - key: iron-key
    primary_name: iron key
    aliases: [key, rusty key]
    kind: Item

  - key: brass-key
    primary_name: brass key
    aliases: [key]
    kind: Item

  - key: rusty-lamp
    primary_name: rusty lamp
    aliases: [lamp]
    kind: Item

  - key: stale-bread
    primary_name: stale bread
    aliases: [bread]
    kind: Item

  - key: cellar-stairs
    primary_name: cellar stairs
    aliases: [stairs]
    kind: Scene
    door:
      direction: north
      to: corridor

  - key: corridor-stairs
    primary_name: cellar stairs
    aliases: [stairs]
    kind: Scene
    door:
      direction: south
      to: cellar

  - key: study-door
    primary_name: study door
    aliases: [door]
    kind: Scene
    door:
      direction: east
      to: study

  - key: wooden-door
    primary_name: wooden door
    aliases: [door]
    kind: Scene
    door:
      direction: west
      to: corridor

  - key: secret-passage
    primary_name: secret passage
    aliases: [passage]
    kind: Scene
    door:
      direction: north
      to: cellar

  - key: oak-door
    primary_name: oak door
    aliases: [door]
    kind: Scene
    door:
      direction: east
      to: corridor
      locked: true

  - key: rusty-nail
    primary_name: rusty nail
    aliases: [nail]
    kind: Item";

const ROOMS_YAML: &str = r"rooms:
  - key: cellar
    name: The Cellar
    visible_objects: [iron-key, brass-key, rusty-lamp, cellar-stairs]
    hidden_objects: [stale-bread]

  - key: corridor
    name: The Corridor
    visible_objects: [corridor-stairs, study-door]
    hidden_objects: []

  - key: study
    name: The Study
    visible_objects: [wooden-door, oak-door]
    hidden_objects: [secret-passage]";

const UNLOCK_AND_EMIT_YAML: &str = r"- verb: use
  item: iron-key
  target:
    kind: scene
  condition:
    - is_door: true
  effect:
    - unlock_exit: east
    - emit: door-unlocked";

const ROOM_GATE_YAML: &str = r"- verb: examine
  target:
    object: iron-key
  condition:
    - room: study
  effect:
    - emit: key-in-study";

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
    use core::world::object::TargetResolution;

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
            TargetResolution::NotFound
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
}

// -----------------------------------------------------------------------
// Dispatch: data interactions authored with keys
// -----------------------------------------------------------------------

mod dispatch {
    use core::{ObjectId, Target, world::object::TargetResolution};

    use super::*;

    #[test]
    fn item_and_target_kind_keys_resolve() {
        let mut engine = engine_with(UNLOCK_AND_EMIT_YAML);
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
    fn room_condition_key_resolves() {
        let mut in_cellar = engine_with(ROOM_GATE_YAML);
        in_cellar.handle_input("take iron key");
        assert_eq!(
            in_cellar.handle_input("examine iron key"),
            vec![Event::Examined {
                target: Target::Object(ObjectId::new("iron-key")),
                target_name: match in_cellar.world().resolve_target("iron key") {
                    TargetResolution::Found(_) => "iron key".to_string(),
                    _ => panic!("iron key resolves"),
                },
            }]
        );

        let mut in_study = engine_with(ROOM_GATE_YAML);
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
        let mut engine = engine_with(
            r"- verb: examine
  target:
    object: wooden-door
  effect:
    - reveal_object: secret-passage",
        );
        iron_key_in_study(&mut engine);
        assert!(engine.world().is_exit_hidden(Direction::North));
        // The effect runs silently (no Examined, no Custom).
        assert_eq!(engine.handle_input("examine wooden door"), vec![]);
        assert!(!engine.world().is_exit_hidden(Direction::North));
        // secret-passage is now resolvable (prove the effect key resolved).
        assert!(matches!(
            engine.world().resolve_target("secret passage"),
            TargetResolution::Found(_)
        ));
    }

    #[test]
    fn drop_effect_with_key_resolves() {
        let mut engine = engine_with(
            r"- verb: drop
  item: brass-key
  effect:
    - emit: key-discarded
    - drop: brass-key",
        );
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
    use core::world::object::TargetResolution;

    use super::*;

    #[test]
    fn interactions_for_reports_compiled_item_key() {
        let mut engine = engine_with(UNLOCK_AND_EMIT_YAML);
        // Carry the brass key too, so "wrong carried object" can be queried.
        engine.handle_input("take brass key");
        iron_key_in_study(&mut engine);
        let ObjectResolution::Found(iron_id) = engine.world().resolve_player_object("iron key")
        else {
            panic!("iron key resolves");
        };
        let TargetResolution::Found(oak_id) = engine.world().resolve_target("oak door") else {
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
        WorldData::from_yaml(&merge_yaml(&[items, rooms, interactions])).map_err(Into::into)
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
  target:
    object: iron-key
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

    // ---- structural minimums ----

    #[test]
    fn world_data_with_no_rooms_is_an_error() {
        let result = from_yaml(ITEMS_YAML, "rooms: []", "{}");
        assert!(result.is_err());
    }
}
