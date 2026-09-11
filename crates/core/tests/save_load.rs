//! Spec for save/load (AGENTS.md north star item 5): a way to persist an
//! in-progress game and resume it later.
//!
//! ## Design: a *progress* snapshot, not a `WorldState` dump
//!
//! `WorldState` is not purely "mutable progress" — it also holds a full copy
//! of the *static* authored content (`data_interactions`, `triggers`,
//! `object_templates`, and every NPC's complete dialogue tree). Serializing
//! `WorldState` wholesale would duplicate that content into every save file
//! and let an old save silently pin a stale copy of it, surviving even after
//! `data/*.yaml` is patched. Instead:
//!
//! * `GameEngine::save(&self) -> Result<String, _>` serializes only the
//!   *dynamic* slice of the game: flags, player inventory, each room's
//!   current visible/hidden object membership, door lock state, fired
//!   triggers, and NPC dialogue progress (`dialogue_state` +
//!   `active_npc`). Human-readable text (YAML, via the `serde_yaml_ng`
//!   dependency `core` already has for `WorldData::from_yaml` — no new
//!   dependency needed), but the exact format is not part of this contract:
//!   nothing here parses the string itself, only round-trips it through
//!   `save`/`load`.
//! * `GameEngine::load(data: &WorldData, rules: impl Rules + 'static, save:
//!   &str) -> Result<GameEngine, _>` rebuilds a `GameEngine` from `data` and
//!   `rules` exactly as `get_with_rules` would (fresh `data_interactions`,
//!   fresh `talk_targets`, fresh NPC dialogue definitions, fresh trigger
//!   definitions), then restores the dynamic slice captured by `save` on top
//!   of it. Passing a *different* `WorldData` than the one `save` was taken
//!   against is not just tolerated, it's the point: a content patch between
//!   save and load must be picked up (new interactions become live; this
//!   suite pins that down under `content_patch_robustness`), while dynamic
//!   progress from the save still applies to whatever of it still exists in
//!   the new data.
//! * The concrete error type `save`/`load` return is an implementation
//!   choice (a dedicated error enum, reusing `WorldDataError`'s shape,
//!   `Box<dyn std::error::Error>`, ...) — this suite only ever checks
//!   `is_ok()`/`is_err()`, never a specific variant.
//!
//! No new `Action`, `Event`, or YAML authoring schema: `save`/`load` are a
//! pure `GameEngine`-level capability, orthogonal to how a game is authored
//! or played (all three front-end modalities can use it identically, or
//! ignore it entirely).
//!
//! Run with: `cargo test --test save_load`.

mod common;

use common::{
    base_world, enter_corridor, enter_study, world_with_interactions as world_with,
    world_with_npcs, world_with_triggers,
};
use core::{
    BasicRules, DialogueNodeId, Direction, Event, GameEngine, NpcId, ObjectId, RoomId, WorldData,
};

// ---------------------------------------------------------------------------
// Round-tripping the dynamic slice of the game
// ---------------------------------------------------------------------------

mod round_trip {
    use super::*;

    #[test]
    fn round_trips_flags_inventory_and_current_room() {
        let data = world_with(
            r"- verb: examine
  target:
    object: rusty-lamp
  effect:
    - set_flag: seen-lamp",
        );
        let mut engine = GameEngine::get(&data);
        engine.handle_input("take brass key");
        engine.handle_input("examine rusty lamp");
        engine.handle_input("go north");

        assert!(engine.world().has_flag("seen-lamp"));
        assert!(engine.world().player_holds(&ObjectId::new("brass-key")));
        assert_eq!(engine.world().current_room_id(), RoomId::new("corridor"));

        let saved = engine.save().expect("save succeeds");
        let reloaded = GameEngine::load(&data, BasicRules, &saved).expect("load succeeds");

        assert!(reloaded.world().has_flag("seen-lamp"));
        assert!(reloaded.world().player_holds(&ObjectId::new("brass-key")));
        assert_eq!(reloaded.world().current_room_id(), RoomId::new("corridor"));
        assert!(
            !reloaded
                .world()
                .room_object_names()
                .contains(&"brass key".to_string())
        );
    }

    #[test]
    fn round_trips_a_grant_only_object_never_placed_in_any_room() {
        // `rusty-nail` is declared in the shared fixture but never listed in
        // any room's `visible_objects`/`hidden_objects` — the only way it
        // ever exists is via a `grant` effect, so a freshly-built
        // `WorldState` has no placed instance of it anywhere to relocate.
        let data = world_with(
            r"- verb: use
  target:
    object: rusty-lamp
  effect:
    - grant: rusty-nail",
        );
        let mut engine = GameEngine::get(&data);
        engine.handle_input("take rusty lamp");
        engine.handle_input("use rusty lamp on rusty lamp");
        assert!(engine.world().player_holds(&ObjectId::new("rusty-nail")));

        let saved = engine.save().expect("save succeeds");
        let reloaded = GameEngine::load(&data, BasicRules, &saved).expect("load succeeds");

        assert!(reloaded.world().player_holds(&ObjectId::new("rusty-nail")));
    }

    #[test]
    fn round_trips_an_unlocked_exit() {
        let data = world_with(
            r"- verb: use
  item: iron-key
  target:
    object: oak-door
  effect:
    - unlock_exit: east",
        );
        let mut engine = GameEngine::get(&data);
        engine.handle_input("take iron key");
        enter_study(&mut engine);
        assert!(engine.world().is_exit_locked(Direction::East));
        engine.handle_input("use iron key on oak door");
        assert!(!engine.world().is_exit_locked(Direction::East));

        let saved = engine.save().expect("save succeeds");
        let reloaded = GameEngine::load(&data, BasicRules, &saved).expect("load succeeds");

        assert_eq!(reloaded.world().current_room_id(), RoomId::new("study"));
        assert!(!reloaded.world().is_exit_locked(Direction::East));
    }

    #[test]
    fn round_trips_a_revealed_hidden_exit() {
        let data = world_with(
            r"- verb: use
  item: iron-key
  target:
    object: wooden-door
  effect:
    - reveal_exit: north",
        );
        let mut engine = GameEngine::get(&data);
        engine.handle_input("take iron key");
        enter_study(&mut engine);
        assert!(engine.world().is_exit_hidden(Direction::North));
        engine.handle_input("use iron key on wooden door");
        assert!(!engine.world().is_exit_hidden(Direction::North));

        let saved = engine.save().expect("save succeeds");
        let reloaded = GameEngine::load(&data, BasicRules, &saved).expect("load succeeds");

        assert!(!reloaded.world().is_exit_hidden(Direction::North));
    }

    #[test]
    fn a_fired_trigger_does_not_refire_after_reload() {
        let data = world_with_triggers(
            r"- key: entered-corridor
  condition:
    - room: corridor
  effect:
    - emit: corridor-beat",
        );
        let mut engine = GameEngine::get(&data);
        let first_entry = engine.handle_input("go north");
        assert!(first_entry.contains(&Event::Custom {
            name: "corridor-beat".to_string()
        }));

        let saved = engine.save().expect("save succeeds");
        let mut reloaded = GameEngine::load(&data, BasicRules, &saved).expect("load succeeds");

        // Leave and re-enter the corridor: if `fired_triggers` hadn't
        // round-tripped, the trigger's condition (room == corridor) would
        // look freshly eligible again and fire a second time.
        reloaded.handle_input("go south");
        let second_entry = reloaded.handle_input("go north");
        assert!(!second_entry.contains(&Event::Custom {
            name: "corridor-beat".to_string()
        }));
    }

    const GUARD_WITH_CHOICE_YAML: &str = r#"npcs:
  - key: guard
    primary_name: guard
    aliases: [sentry]
    room: corridor
    dialogue:
      root: greeting
      nodes:
        greeting:
          text: "Halt! Who goes there?"
          choices:
            - label: A friend
              next: friend-response
        friend-response:
          text: "Pass, friend.""#;

    #[test]
    fn round_trips_mid_conversation_dialogue_progress() {
        let data = world_with_npcs(GUARD_WITH_CHOICE_YAML);
        let mut engine = GameEngine::get(&data);
        enter_corridor(&mut engine);
        engine.handle_input("talk guard");

        let saved = engine.save().expect("save succeeds");
        let mut reloaded = GameEngine::load(&data, BasicRules, &saved).expect("load succeeds");

        // Continuing the conversation only works if `dialogue_state` and
        // `active_npc` came back with the save, not reset to "no active
        // conversation".
        assert_eq!(
            reloaded.handle_input("choose 1"),
            vec![
                Event::Talked {
                    npc_id: NpcId::new("guard"),
                    npc: "guard".to_string(),
                    node_id: DialogueNodeId::new("friend-response"),
                    text: "Pass, friend.".to_string(),
                    choices: Vec::new(),
                },
                Event::DialogueEnded {
                    npc_id: NpcId::new("guard"),
                    npc: "guard".to_string(),
                },
            ]
        );
    }
}

// ---------------------------------------------------------------------------
// Loading a save against *different* `WorldData` than it was taken against
// (the reason this is a progress-only snapshot rather than a `WorldState`
// dump in the first place)
// ---------------------------------------------------------------------------

mod content_patch_robustness {
    use super::*;

    #[test]
    fn a_reloaded_save_uses_the_interactions_of_the_data_passed_to_load_not_save() {
        let original = world_with("");
        let mut engine = GameEngine::get(&original);
        engine.handle_input("take brass key");
        let saved = engine.save().expect("save succeeds");

        // A content patch made after the save, adding an interaction that
        // did not exist in `original`.
        let patched = world_with(
            r"- verb: examine
  target:
    object: rusty-lamp
  effect:
    - emit: patched-in-after-save",
        );
        let mut reloaded = GameEngine::load(&patched, BasicRules, &saved).expect("load succeeds");

        // Dynamic progress from the old save still applies...
        assert!(reloaded.world().player_holds(&ObjectId::new("brass-key")));
        // ...but the interaction that fires is `patched`'s, proving the
        // engine isn't running a copy of `original`'s (nonexistent) content
        // frozen at save time.
        assert_eq!(
            reloaded.handle_input("examine rusty lamp"),
            vec![Event::Custom {
                name: "patched-in-after-save".to_string()
            }]
        );
    }

    #[test]
    fn a_save_referencing_an_object_removed_by_a_later_patch_still_loads() {
        let v1 = WorldData::from_yaml(
            r"
objects:
  - key: torch
    primary_name: torch
rooms:
  - key: room
    visible_objects: [torch]
",
        )
        .expect("v1 world parses");
        let mut engine = GameEngine::get(&v1);
        engine.handle_input("take torch");
        let saved = engine.save().expect("save succeeds");

        // The patch drops `torch` (and its room reference) entirely.
        let v2 = WorldData::from_yaml(
            r"
objects: []
rooms:
  - key: room
    visible_objects: []
",
        )
        .expect("v2 world parses");

        let reloaded = GameEngine::load(&v2, BasicRules, &saved);
        assert!(reloaded.is_ok());
    }
}

// ---------------------------------------------------------------------------
// Error handling and purity
// ---------------------------------------------------------------------------

mod malformed_save {
    use super::*;

    #[test]
    fn loading_a_malformed_save_string_is_an_error() {
        let data = base_world();
        let result = GameEngine::load(&data, BasicRules, "not: [valid, yaml: structure");
        assert!(result.is_err());
    }
}

mod read_only_query {
    use super::*;

    #[test]
    fn save_does_not_mutate_the_engine_and_is_idempotent() {
        let data = world_with(
            r"- verb: examine
  target:
    object: rusty-lamp
  effect:
    - set_flag: seen-lamp",
        );
        // A non-`mut` binding: `save` compiling here at all is part of the
        // contract (`&self`, never `&mut self`).
        let engine = GameEngine::get(&data);
        let first = engine.save().expect("save succeeds");
        let second = engine.save().expect("save succeeds");
        assert_eq!(first, second);
    }
}
