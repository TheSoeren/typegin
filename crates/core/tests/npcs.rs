//! NPCs and dialogue trees.
//!
//! A first-class NPC model with data-driven dialogue graphs: branching
//! conversations authored in YAML, dispatched by a `Talk` action, with
//! choices that advance the conversation and can run effects (set flags,
//! emit beats, grant items).
//!
//! ## Schema additions
//!
//! ### NPCs file (`data/npcs.yaml` or `from_yaml` fifth parameter)
//!
//! ```yaml
//! npcs:
//!   - key: guard
//!     primary_name: guard
//!     aliases: [sentry]
//!     room: corridor          # room key where the NPC lives
//!     dialogue:
//!       root: greeting        # starting node id
//!       nodes:
//!         greeting:
//!           text: "Hello there."
//!           choices:
//!             - label: Ask about the key
//!               next: about-key
//!               effect:
//!                 - set_flag: guard-told-about-key
//!                 - emit: key-hint
//!         about-key:
//!           text: "The key is in the cellar."
//! ```
//!
//! Node ids are [`DialogueNodeId`]s; a choice's `next` may be `.end` to
//! terminate the conversation. Choices may declare a stable `key`
//! ([`DialogueOptionId`]) for cross-run identification by a point-and-click UI.
//!
//! ### Actions
//!
//! * `Action::Talk(String)` — `talk to <npc>` / `talk <npc>` (stop-word
//!   "to" is stripped by the tokenizer).
//! * `Action::Choose(String)` — `choose <label>` / `choose <index>` where
//!   index is 1-based.
//!
//! ### Events
//!
//! * `Event::Talked { npc_id, npc, node_id: DialogueNodeId, text, choices }`
//!   — the NPC's dialogue line plus the player's available choices.
//! * `Event::DialogueEnded { npc_id, npc }` — conversation ended (a node
//!   with no choices, or a `.end` choice).
//! * `Event::TalkNpcNotFound { npc }` — no NPC by that name in the room.
//! * `Event::DialogueInvalidChoice { npc, choice }` — choice didn't match.
//!
//! ## Dispatch flow
//!
//! 1. `GameEngine::handle_input("talk guard")` → `Action::Talk("guard")`.
//! 2. Engine resolves the NPC by name in the current room.
//! 3. If found: marks it `active`, sets `dialogue_state[npc_id] = root`,
//!    returns `Talked`.
//! 4. If not found: returns `TalkNpcNotFound`.
//!
//! A `Talked` node with no choices immediately also emits `DialogueEnded`
//! and clears the conversation (nothing left to choose).
//!
//! Choice flow:
//! 1. `GameEngine::handle_input("choose 1")` → `Action::Choose("1")`.
//! 2. Engine matches the active NPC's current choices by index (1-based) or
//!    label (case-insensitive).
//! 3. Runs the choice's effects in order, then advances to `next`.
//! 4. If `next` is `.end`: clears the conversation, returns `DialogueEnded`.
//! 5. Otherwise: returns `Talked` for the next node.
//!
//! Choose with no active dialogue is `UnknownEvent`; a non-matching choice is
//! `DialogueInvalidChoice`.
//!
//! ## State management
//!
//! * `dialogue_state: HashMap<NpcId, DialogueNodeId>` on `WorldState` tracks
//!   the current dialogue node per NPC.
//! * `WorldState::active_npc: Option<NpcId>` is who `Choose` dispatch reads.
//! * Moving rooms (`Action::Go`) clears the active dialogue.
//! * Ending a conversation (reaching `.end` or a no-choice node) clears the
//!   NPC's dialogue state and the active NPC.
//!
//! Run with: `cargo test --test npcs`.

mod common;

use core::{
    Action, DataEffect, DialogueChoice, DialogueNodeId, Direction, Event, GameEngine, NpcId,
    ObjectId, RoomId, WorldData, WorldDataError,
};

const ITEMS_YAML: &str = include_str!("../data/data_interactions_items.yaml");
const ROOMS_YAML: &str = include_str!("../data/data_interactions_rooms.yaml");
const GLOBALS_YAML: &str = "{}";

fn base_world() -> WorldData {
    WorldData::from_yaml(GLOBALS_YAML, ITEMS_YAML, ROOMS_YAML, "{}", "{}")
        .expect("base fixture parses")
}

fn world_with(npcs: &str) -> WorldData {
    WorldData::from_yaml(GLOBALS_YAML, ITEMS_YAML, ROOMS_YAML, "{}", npcs)
        .expect("fixture with npcs parses")
}

fn world_with_npc_and_interactions(npcs: &str, interactions: &str) -> WorldData {
    let interactions_yaml = format!("interactions:\n{interactions}");
    WorldData::from_yaml(
        GLOBALS_YAML,
        ITEMS_YAML,
        ROOMS_YAML,
        &interactions_yaml,
        npcs,
    )
    .expect("fixture with npcs and interactions parses")
}

fn engine_with(npcs: &str) -> GameEngine {
    GameEngine::get(&world_with(npcs))
}

fn enter_corridor(engine: &mut GameEngine) {
    assert_eq!(
        engine.handle_input("go north"),
        vec![Event::Went(Direction::North)]
    );
    assert_eq!(engine.world().current_room_id(), RoomId::new("corridor"));
}

// ---------------------------------------------------------------------------
// WorldState API
// ---------------------------------------------------------------------------

mod world_state_api {
    use super::*;

    #[test]
    fn npcs_start_empty() {
        let engine = GameEngine::get(&base_world());
        assert!(engine.world().npcs().is_empty());
    }

    #[test]
    fn npcs_are_loaded_from_data() {
        let engine = engine_with(include_str!("../data/npcs_guard.yaml"));
        assert_eq!(engine.world().npcs().len(), 1);
        assert_eq!(engine.world().npcs()[0].id(), &NpcId::new("guard"));
    }

    #[test]
    fn npc_names_are_loaded() {
        let engine = engine_with(include_str!("../data/npcs_guard.yaml"));
        assert_eq!(engine.world().npcs()[0].primary_name(), "guard");
    }

    #[test]
    fn npc_aliases_are_loaded() {
        let engine = engine_with(include_str!("../data/npcs_guard.yaml"));
        let aliases = engine.world().npcs()[0].aliases();
        assert!(aliases.contains(&"sentry".to_string()));
    }

    #[test]
    fn npc_room_is_loaded() {
        let engine = engine_with(include_str!("../data/npcs_guard.yaml"));
        assert_eq!(engine.world().npcs()[0].room_id(), &RoomId::new("corridor"));
    }

    #[test]
    fn npc_root_node_is_loaded() {
        let engine = engine_with(include_str!("../data/npcs_guard.yaml"));
        assert_eq!(
            engine.world().npcs()[0].root(),
            &DialogueNodeId::new("greeting")
        );
    }

    #[test]
    fn dialogue_graph_is_loaded() {
        let engine = engine_with(include_str!("../data/npcs_guard.yaml"));
        let npc = &engine.world().npcs()[0];
        assert!(
            npc.dialogue()
                .contains_key(&DialogueNodeId::new("greeting"))
        );
    }

    #[test]
    fn dialogue_node_text_is_loaded() {
        let engine = engine_with(include_str!("../data/npcs_guard.yaml"));
        let npc = &engine.world().npcs()[0];
        let node = npc
            .dialogue()
            .get(&DialogueNodeId::new("greeting"))
            .expect("root node exists");
        assert_eq!(node.text(), "The guard nods at you.");
    }

    #[test]
    fn dialogue_choice_next_is_typed() {
        let engine = engine_with(include_str!("../data/npcs_two_branches.yaml"));
        let npc = &engine.world().npcs()[0];
        let node = npc
            .dialogue()
            .get(&DialogueNodeId::new("greeting"))
            .expect("root node exists");
        let choices = node.choices();
        assert_eq!(choices[0].label(), "Ask about the exit");
        assert_eq!(choices[0].next(), Some(&DialogueNodeId::new("about-exit")));
        // `.end` marker maps to `None`
        assert_eq!(choices[1].next(), None);
    }

    #[test]
    fn dialogue_state_starts_empty() {
        let engine = GameEngine::get(&base_world());
        assert!(engine.world().dialogue_state().is_empty());
    }

    #[test]
    fn active_npc_starts_none() {
        let engine = GameEngine::get(&base_world());
        assert!(engine.world().active_npc().is_none());
    }

    #[test]
    fn find_npc_by_name_in_room() {
        let engine = engine_with(include_str!("../data/npcs_guard.yaml"));
        let corridor_npcs = engine.world().npcs_in_room(&RoomId::new("corridor"));
        assert_eq!(corridor_npcs.len(), 1);
        assert_eq!(corridor_npcs[0].id(), &NpcId::new("guard"));
    }

    #[test]
    fn find_npc_by_alias_in_room() {
        let engine = engine_with(include_str!("../data/npcs_guard.yaml"));
        let corridor_npcs = engine.world().npcs_in_room(&RoomId::new("corridor"));
        let found = corridor_npcs.iter().find(|npc| npc.has_name("sentry"));
        assert!(found.is_some());
    }

    #[test]
    fn resolve_npc_by_alias() {
        let mut engine = engine_with(include_str!("../data/npcs_guard.yaml"));
        enter_corridor(&mut engine);
        let found = engine.world().resolve_npc("sentry");
        assert!(found.is_some());
        assert_eq!(found.unwrap().id(), &NpcId::new("guard"));
    }

    #[test]
    fn resolve_npc_unknown_name_is_none() {
        let engine = engine_with(include_str!("../data/npcs_guard.yaml"));
        assert!(engine.world().resolve_npc("ghost").is_none());
    }

    #[test]
    fn npc_not_in_other_room() {
        let engine = engine_with(include_str!("../data/npcs_guard.yaml"));
        let cellar_npcs = engine.world().npcs_in_room(&RoomId::new("cellar"));
        assert!(cellar_npcs.is_empty());
    }

    #[test]
    fn two_npcs_in_same_room() {
        let engine = engine_with(include_str!("../data/npcs_two_npcs.yaml"));
        let corridor_npcs = engine.world().npcs_in_room(&RoomId::new("corridor"));
        assert_eq!(corridor_npcs.len(), 2);
    }
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

mod parse {
    use super::*;

    #[test]
    fn single_npc_parses() {
        let world = world_with(include_str!("../data/npcs_guard.yaml"));
        assert_eq!(world.npcs.len(), 1);
    }

    #[test]
    fn npc_key_is_parsed_as_npc_id() {
        let world = world_with(include_str!("../data/npcs_guard.yaml"));
        assert_eq!(world.npcs[0].id, NpcId::new("guard"));
    }

    #[test]
    fn npc_primary_name_is_parsed() {
        let world = world_with(include_str!("../data/npcs_guard.yaml"));
        assert_eq!(world.npcs[0].primary_name, "guard");
    }

    #[test]
    fn npc_aliases_are_parsed() {
        let world = world_with(include_str!("../data/npcs_guard.yaml"));
        assert_eq!(world.npcs[0].aliases, vec!["sentry"]);
    }

    #[test]
    fn npc_room_is_parsed() {
        let world = world_with(include_str!("../data/npcs_guard.yaml"));
        assert_eq!(world.npcs[0].room, RoomId::new("corridor"));
    }

    #[test]
    fn dialogue_root_is_parsed() {
        let world = world_with(include_str!("../data/npcs_guard.yaml"));
        assert_eq!(world.npcs[0].dialogue.root, "greeting");
    }

    #[test]
    fn dialogue_node_text_is_parsed() {
        let world = world_with(include_str!("../data/npcs_guard.yaml"));
        let node = world.npcs[0]
            .dialogue
            .nodes
            .get("greeting")
            .expect("node exists");
        assert_eq!(node.text, "The guard nods at you.");
    }

    #[test]
    fn dialogue_node_without_choices_parses() {
        let world = world_with(include_str!("../data/npcs_guard.yaml"));
        let node = world.npcs[0]
            .dialogue
            .nodes
            .get("greeting")
            .expect("node exists");
        assert!(node.choices.is_empty());
    }

    #[test]
    fn dialogue_node_with_choices_parses() {
        let world = world_with(include_str!("../data/npcs_two_branches.yaml"));
        let node = world.npcs[0]
            .dialogue
            .nodes
            .get("greeting")
            .expect("node exists");
        assert_eq!(node.choices.len(), 2);
    }

    #[test]
    fn dialogue_choice_label_is_parsed() {
        let world = world_with(include_str!("../data/npcs_two_branches.yaml"));
        let node = world.npcs[0]
            .dialogue
            .nodes
            .get("greeting")
            .expect("node exists");
        assert_eq!(node.choices[0].label, "Ask about the exit");
    }

    #[test]
    fn dialogue_choice_next_is_parsed() {
        let world = world_with(include_str!("../data/npcs_two_branches.yaml"));
        let node = world.npcs[0]
            .dialogue
            .nodes
            .get("greeting")
            .expect("node exists");
        assert_eq!(node.choices[0].next, "about-exit");
    }

    #[test]
    fn dialogue_choice_end_marker_is_parsed() {
        let world = world_with(include_str!("../data/npcs_two_branches.yaml"));
        let node = world.npcs[0]
            .dialogue
            .nodes
            .get("greeting")
            .expect("node exists");
        assert_eq!(node.choices[1].next, ".end");
    }

    #[test]
    fn dialogue_choice_key_defaults_to_none() {
        let world = world_with(include_str!("../data/npcs_two_branches.yaml"));
        let node = world.npcs[0]
            .dialogue
            .nodes
            .get("greeting")
            .expect("node exists");
        assert_eq!(node.choices[0].option_id, None);
    }

    #[test]
    fn dialogue_choice_effects_are_parsed() {
        let world = world_with(include_str!("../data/npcs_effects.yaml"));
        let node = world.npcs[0]
            .dialogue
            .nodes
            .get("greeting")
            .expect("node exists");
        assert_eq!(node.choices[0].effect.len(), 2);
        assert!(node.choices[0].effect.contains(&DataEffect::SetFlag {
            flag: "guard-told-about-key".to_string()
        }));
    }

    #[test]
    fn two_npcs_parse() {
        let world = world_with(include_str!("../data/npcs_two_npcs.yaml"));
        assert_eq!(world.npcs.len(), 2);
        assert_eq!(world.npcs[0].id, NpcId::new("guard"));
        assert_eq!(world.npcs[1].id, NpcId::new("warden"));
    }

    #[test]
    fn empty_npcs_yaml_parses_to_empty() {
        let world = WorldData::from_yaml(GLOBALS_YAML, ITEMS_YAML, ROOMS_YAML, "{}", "{}")
            .expect("empty npcs parses");
        assert!(world.npcs.is_empty());
    }

    #[test]
    fn npc_references_unknown_room_is_a_validation_error() {
        let result = WorldData::from_yaml(
            GLOBALS_YAML,
            ITEMS_YAML,
            ROOMS_YAML,
            "{}",
            "npcs:\n  - key: ghost\n    primary_name: ghost\n    room: nonroom\n    dialogue:\n      root: hi\n      nodes:\n        hi:\n          text: Boo\n",
        );
        assert!(matches!(result, Err(WorldDataError::Validation(_))));
    }

    #[test]
    fn dialogue_references_unknown_node_is_a_validation_error() {
        let result = WorldData::from_yaml(
            GLOBALS_YAML,
            ITEMS_YAML,
            ROOMS_YAML,
            "{}",
            "npcs:\n  - key: guard\n    primary_name: guard\n    room: corridor\n    dialogue:\n      root: missing\n      nodes:\n        hi:\n          text: Hello\n",
        );
        assert!(matches!(result, Err(WorldDataError::Validation(_))));
    }

    #[test]
    fn dialogue_choice_references_unknown_node_is_a_validation_error() {
        let result = WorldData::from_yaml(
            GLOBALS_YAML,
            ITEMS_YAML,
            ROOMS_YAML,
            "{}",
            "npcs:\n  - key: guard\n    primary_name: guard\n    room: corridor\n    dialogue:\n      root: hi\n      nodes:\n        hi:\n          text: Hello\n          choices:\n            - label: Go\n              next: nonexist\n",
        );
        assert!(matches!(result, Err(WorldDataError::Validation(_))));
    }
}

// ---------------------------------------------------------------------------
// Input parsing: Action::Talk and Action::Choose
// ---------------------------------------------------------------------------

mod input_parse {
    use super::*;
    use core::parse_input;

    #[test]
    fn talk_to_npc_parses() {
        assert_eq!(
            parse_input("talk to guard"),
            Action::Talk("guard".to_string())
        );
    }

    #[test]
    fn talk_without_to_parses() {
        assert_eq!(parse_input("talk guard"), Action::Talk("guard".to_string()));
    }

    #[test]
    fn talk_with_multi_word_npc_parses() {
        assert_eq!(
            parse_input("talk to old man"),
            Action::Talk("old man".to_string())
        );
    }

    #[test]
    fn choose_index_parses() {
        assert_eq!(parse_input("choose 1"), Action::Choose("1".to_string()));
    }

    #[test]
    fn choose_label_parses() {
        // Stop words are stripped by the tokenizer.
        assert_eq!(
            parse_input("choose ask about the exit"),
            Action::Choose("ask exit".to_string())
        );
    }

    #[test]
    fn talk_without_target_is_unknown() {
        assert!(matches!(parse_input("talk"), Action::Unknown(_)));
    }

    #[test]
    fn choose_without_payload_is_unknown() {
        assert!(matches!(parse_input("choose"), Action::Unknown(_)));
    }
}

// ---------------------------------------------------------------------------
// Talk dispatch
// ---------------------------------------------------------------------------

mod talk_dispatch {
    use super::*;

    #[test]
    fn talk_to_npc_returns_dialogue_node() {
        let mut engine = engine_with(include_str!("../data/npcs_guard.yaml"));
        enter_corridor(&mut engine);
        // No choices → the line is spoken, then the conversation ends.
        assert_eq!(
            engine.handle_input("talk to guard"),
            vec![
                Event::Talked {
                    npc_id: NpcId::new("guard"),
                    npc: "guard".to_string(),
                    node_id: DialogueNodeId::new("greeting"),
                    text: "The guard nods at you.".to_string(),
                    choices: vec![],
                },
                Event::DialogueEnded {
                    npc_id: NpcId::new("guard"),
                    npc: "guard".to_string(),
                },
            ]
        );
    }

    #[test]
    fn talk_to_npc_not_in_room_returns_not_found() {
        let mut engine = engine_with(include_str!("../data/npcs_guard.yaml"));
        // Still in the cellar, guard is in corridor
        assert_eq!(
            engine.handle_input("talk to guard"),
            vec![Event::TalkNpcNotFound {
                npc: "guard".to_string(),
            }]
        );
    }

    #[test]
    fn talk_to_unknown_name_returns_not_found() {
        let mut engine = engine_with(include_str!("../data/npcs_guard.yaml"));
        enter_corridor(&mut engine);
        assert_eq!(
            engine.handle_input("talk to ghost"),
            vec![Event::TalkNpcNotFound {
                npc: "ghost".to_string(),
            }]
        );
    }

    #[test]
    fn talk_sets_dialogue_state_and_active_npc() {
        let mut engine = engine_with(include_str!("../data/npcs_two_branches.yaml"));
        enter_corridor(&mut engine);
        engine.handle_input("talk to guard");
        assert_eq!(
            engine.world().dialogue_state().get(&NpcId::new("guard")),
            Some(&DialogueNodeId::new("greeting"))
        );
        assert_eq!(engine.world().active_npc(), &Some(NpcId::new("guard")));
    }

    #[test]
    fn talk_node_with_choices_returns_them() {
        let mut engine = engine_with(include_str!("../data/npcs_two_branches.yaml"));
        enter_corridor(&mut engine);
        let events = engine.handle_input("talk to guard");
        match &events[0] {
            Event::Talked { choices, .. } => {
                assert_eq!(choices.len(), 2);
                assert_eq!(choices[0].label, "Ask about the exit");
                assert_eq!(choices[0].next, Some(DialogueNodeId::new("about-exit")));
                assert_eq!(choices[1].label, "Say nothing");
                assert_eq!(choices[1].next, None);
            }
            other => panic!("expected Talked, got {other:?}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Choose dispatch
// ---------------------------------------------------------------------------

mod choose_dispatch {
    use super::*;

    #[test]
    fn choose_by_index_advances_to_next_node() {
        let mut engine = engine_with(include_str!("../data/npcs_two_branches.yaml"));
        enter_corridor(&mut engine);
        engine.handle_input("talk to guard");
        // `about-exit` is a leaf node → line spoken, then the conversation ends.
        assert_eq!(
            engine.handle_input("choose 1"),
            vec![
                Event::Talked {
                    npc_id: NpcId::new("guard"),
                    npc: "guard".to_string(),
                    node_id: DialogueNodeId::new("about-exit"),
                    text: "The exit is to the north.".to_string(),
                    choices: vec![],
                },
                Event::DialogueEnded {
                    npc_id: NpcId::new("guard"),
                    npc: "guard".to_string(),
                },
            ]
        );
    }

    #[test]
    fn choose_end_marker_ends_dialogue() {
        let mut engine = engine_with(include_str!("../data/npcs_two_branches.yaml"));
        enter_corridor(&mut engine);
        engine.handle_input("talk to guard");
        assert_eq!(
            engine.handle_input("choose 2"),
            vec![Event::DialogueEnded {
                npc_id: NpcId::new("guard"),
                npc: "guard".to_string(),
            }]
        );
    }

    #[test]
    fn choose_clears_dialogue_state() {
        let mut engine = engine_with(include_str!("../data/npcs_two_branches.yaml"));
        enter_corridor(&mut engine);
        engine.handle_input("talk to guard");
        engine.handle_input("choose 2");
        assert!(
            !engine
                .world()
                .dialogue_state()
                .contains_key(&NpcId::new("guard"))
        );
        assert!(engine.world().active_npc().is_none());
    }

    #[test]
    fn choose_by_label_ends_dialogue() {
        let mut engine = engine_with(include_str!("../data/npcs_two_branches.yaml"));
        enter_corridor(&mut engine);
        engine.handle_input("talk to guard");
        assert_eq!(
            engine.handle_input("choose say nothing"),
            vec![Event::DialogueEnded {
                npc_id: NpcId::new("guard"),
                npc: "guard".to_string(),
            }]
        );
    }

    #[test]
    fn choose_invalid_index_returns_error() {
        let mut engine = engine_with(include_str!("../data/npcs_two_branches.yaml"));
        enter_corridor(&mut engine);
        engine.handle_input("talk to guard");
        assert_eq!(
            engine.handle_input("choose 99"),
            vec![Event::DialogueInvalidChoice {
                npc: "guard".to_string(),
                choice: "99".to_string(),
            }]
        );
    }

    #[test]
    fn choose_invalid_label_returns_error() {
        let mut engine = engine_with(include_str!("../data/npcs_two_branches.yaml"));
        enter_corridor(&mut engine);
        engine.handle_input("talk to guard");
        assert_eq!(
            engine.handle_input("choose nonexistent"),
            vec![Event::DialogueInvalidChoice {
                npc: "guard".to_string(),
                choice: "nonexistent".to_string(),
            }]
        );
    }

    #[test]
    fn choose_without_active_dialogue_is_unknown() {
        let mut engine = engine_with(include_str!("../data/npcs_two_branches.yaml"));
        enter_corridor(&mut engine);
        let events = engine.handle_input("choose 1");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], Event::UnknownEvent { .. }));
    }

    #[test]
    fn choose_effects_are_applied() {
        let mut engine = engine_with(include_str!("../data/npcs_effects.yaml"));
        enter_corridor(&mut engine);
        engine.handle_input("talk to guard");
        assert!(!engine.world().has_flag("guard-told-about-key"));
        engine.handle_input("choose 1");
        assert!(engine.world().has_flag("guard-told-about-key"));
    }

    #[test]
    fn choose_effects_emit_events() {
        let mut engine = engine_with(include_str!("../data/npcs_effects.yaml"));
        enter_corridor(&mut engine);
        engine.handle_input("talk to guard");
        let events = engine.handle_input("choose 1");
        // Effects run first, then the next dialogue node.
        assert!(
            events.contains(&Event::Custom {
                name: "guard-told-beat".to_string()
            }),
            "expected Custom beat in events: {events:?}"
        );
        assert!(
            events.contains(&Event::Talked {
                npc_id: NpcId::new("guard"),
                npc: "guard".to_string(),
                node_id: DialogueNodeId::new("about-escape"),
                text: "Don't try it.".to_string(),
                choices: vec![],
            }),
            "expected next node's Talked in events: {events:?}"
        );
    }

    #[test]
    fn deep_chain_walks_three_nodes() {
        let mut engine = engine_with(include_str!("../data/npcs_deep_chain.yaml"));
        enter_corridor(&mut engine);

        // Node 1: greeting
        let events = engine.handle_input("talk to guard");
        match &events[0] {
            Event::Talked {
                node_id,
                text,
                choices,
                ..
            } => {
                assert_eq!(node_id, &DialogueNodeId::new("greeting"));
                assert_eq!(text, "The guard looks at you.");
                assert_eq!(choices.len(), 1);
            }
            other => panic!("expected Talked, got {other:?}"),
        }

        // Node 2: ask-key (has a choice → no end)
        let events = engine.handle_input("choose 1");
        match &events[0] {
            Event::Talked { node_id, text, .. } => {
                assert_eq!(node_id, &DialogueNodeId::new("ask-key"));
                assert_eq!(text, "What key?");
            }
            other => panic!("expected Talked, got {other:?}"),
        }
        assert_eq!(
            engine.world().dialogue_state().get(&NpcId::new("guard")),
            Some(&DialogueNodeId::new("ask-key"))
        );

        // Node 3: answer-key (leaf → line + end)
        let events = engine.handle_input("choose 1");
        match &events[0] {
            Event::Talked {
                node_id,
                text,
                choices,
                ..
            } => {
                assert_eq!(node_id, &DialogueNodeId::new("answer-key"));
                assert_eq!(text, "Oh, that key. It's in the cellar.");
                assert!(choices.is_empty());
            }
            other => panic!("expected Talked, got {other:?}"),
        }
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::DialogueEnded { .. }))
        );
        assert!(engine.world().active_npc().is_none());
    }
}

// ---------------------------------------------------------------------------
// State management
// ---------------------------------------------------------------------------

mod state_management {
    use super::*;

    #[test]
    fn moving_rooms_clears_active_dialogue() {
        let mut engine = engine_with(include_str!("../data/npcs_two_branches.yaml"));
        enter_corridor(&mut engine);
        engine.handle_input("talk to guard");
        assert!(engine.world().active_npc().is_some());
        // Move back to cellar
        assert_eq!(
            engine.handle_input("go south"),
            vec![Event::Went(Direction::South)]
        );
        assert!(engine.world().active_npc().is_none());
        assert!(engine.world().dialogue_state().is_empty());
    }

    #[test]
    fn talk_to_different_npc_replaces_active() {
        let mut engine = engine_with(include_str!("../data/npcs_two_npcs.yaml"));
        enter_corridor(&mut engine);
        engine.handle_input("talk to guard");
        assert_eq!(engine.world().active_npc(), &Some(NpcId::new("guard")));
        assert_eq!(
            engine.world().dialogue_state().get(&NpcId::new("guard")),
            Some(&DialogueNodeId::new("greeting"))
        );
        engine.handle_input("talk to warden");
        assert_eq!(engine.world().active_npc(), &Some(NpcId::new("warden")));
        assert_eq!(
            engine.world().dialogue_state().get(&NpcId::new("warden")),
            Some(&DialogueNodeId::new("greeting"))
        );
    }

    #[test]
    fn ending_dialogue_allows_new_conversation() {
        let mut engine = engine_with(include_str!("../data/npcs_two_branches.yaml"));
        enter_corridor(&mut engine);
        engine.handle_input("talk to guard");
        engine.handle_input("choose 2"); // ends dialogue
        // Start a new conversation from the root.
        let events = engine.handle_input("talk to guard");
        match &events[0] {
            Event::Talked { node_id, .. } => {
                assert_eq!(node_id, &DialogueNodeId::new("greeting"));
            }
            other => panic!("expected Talked, got {other:?}"),
        }
    }
}

// ---------------------------------------------------------------------------
// View hooks
// ---------------------------------------------------------------------------

mod view_hooks {
    use super::*;
    use core::{RenderCommand, View};

    struct TestView;

    impl View for TestView {
        fn render_talked(
            &mut self,
            npc: &str,
            text: &str,
            choices: &[core::event::DialogueChoice],
        ) -> Vec<RenderCommand> {
            let mut out = vec![RenderCommand::Line(format!("{npc}: {text}"))];
            for (i, choice) in choices.iter().enumerate() {
                out.push(RenderCommand::Line(format!(
                    "  {}. {}",
                    i + 1,
                    choice.label
                )));
            }
            out
        }

        fn render_dialogue_ended(&mut self, npc: &str) -> Vec<RenderCommand> {
            vec![RenderCommand::Line(format!("{npc} conversation ended."))]
        }

        fn render_talk_npc_not_found(&mut self, npc: &str) -> Vec<RenderCommand> {
            vec![RenderCommand::Line(format!("There is no {npc} here."))]
        }
    }

    // These drive `View::render` directly with crafted events, so the view
    // dispatch contract is pinned independent of the (not yet implemented)
    // dialogue rules.

    #[test]
    fn talked_renders_npc_text_and_choices() {
        let engine = GameEngine::get(&base_world());
        let world = engine.world();
        let events = vec![Event::Talked {
            npc_id: NpcId::new("guard"),
            npc: "guard".to_string(),
            node_id: DialogueNodeId::new("greeting"),
            text: "The guard looks at you.".to_string(),
            choices: vec![
                DialogueChoice {
                    option_id: None,
                    label: "Ask about the exit".to_string(),
                    next: Some(DialogueNodeId::new("about-exit")),
                },
                DialogueChoice {
                    option_id: None,
                    label: "Say nothing".to_string(),
                    next: None,
                },
            ],
        }];
        let mut view = TestView;
        let commands = view.render(&events, world);
        assert_eq!(commands.len(), 3);
        match &commands[0] {
            RenderCommand::Line(text) => assert_eq!(text, "guard: The guard looks at you."),
            _ => panic!("expected first command to be Line"),
        }
        match &commands[1] {
            RenderCommand::Line(text) => assert_eq!(text, "  1. Ask about the exit"),
            _ => panic!("expected second command to be Line"),
        }
        match &commands[2] {
            RenderCommand::Line(text) => assert_eq!(text, "  2. Say nothing"),
            _ => panic!("expected third command to be Line"),
        }
    }

    #[test]
    fn talked_renders_choice_option_ids() {
        let engine = GameEngine::get(&base_world());
        let world = engine.world();
        let events = vec![Event::Talked {
            npc_id: NpcId::new("guard"),
            npc: "guard".to_string(),
            node_id: DialogueNodeId::new("greeting"),
            text: "The guard looks at you.".to_string(),
            choices: vec![DialogueChoice {
                option_id: Some(core::DialogueOptionId::new("ask-exit")),
                label: "Ask about the exit".to_string(),
                next: Some(DialogueNodeId::new("about-exit")),
            }],
        }];
        let mut view = TestView;
        let commands = view.render(&events, world);
        assert_eq!(commands.len(), 2);
    }

    #[test]
    fn dialogue_ended_renders_end_message() {
        let engine = GameEngine::get(&base_world());
        let world = engine.world();
        let events = vec![Event::DialogueEnded {
            npc_id: NpcId::new("guard"),
            npc: "guard".to_string(),
        }];
        let mut view = TestView;
        let commands = view.render(&events, world);
        assert_eq!(commands.len(), 1);
        match &commands[0] {
            RenderCommand::Line(text) => assert_eq!(text, "guard conversation ended."),
            _ => panic!("expected Line"),
        }
    }

    #[test]
    fn talk_not_found_renders_error() {
        let engine = GameEngine::get(&base_world());
        let world = engine.world();
        let events = vec![Event::TalkNpcNotFound {
            npc: "ghost".to_string(),
        }];
        let mut view = TestView;
        let commands = view.render(&events, world);
        assert_eq!(commands.len(), 1);
        match &commands[0] {
            RenderCommand::Line(text) => assert_eq!(text, "There is no ghost here."),
            _ => panic!("expected Line"),
        }
    }

    #[test]
    fn invalid_choice_renders_error() {
        struct InvalidView;
        impl View for InvalidView {
            fn render_dialogue_invalid_choice(
                &mut self,
                npc: &str,
                choice: &str,
            ) -> Vec<RenderCommand> {
                vec![RenderCommand::Line(format!(
                    "{npc}: no such choice \"{choice}\""
                ))]
            }
        }

        let engine = GameEngine::get(&base_world());
        let world = engine.world();
        let events = vec![Event::DialogueInvalidChoice {
            npc: "guard".to_string(),
            choice: "99".to_string(),
        }];
        let mut view = InvalidView;
        let commands = view.render(&events, world);
        assert_eq!(commands.len(), 1);
        match &commands[0] {
            RenderCommand::Line(text) => assert_eq!(text, "guard: no such choice \"99\""),
            _ => panic!("expected Line"),
        }
    }
}

// ---------------------------------------------------------------------------
// Integration: dialogue effects gate item interactions
// ---------------------------------------------------------------------------

mod integration {
    use core::Target;

    use super::*;

    #[test]
    fn dialogue_set_flag_gates_data_interaction() {
        let mut engine = GameEngine::get(&world_with_npc_and_interactions(
            include_str!("../data/npcs_effects.yaml"),
            include_str!("../data/interactions/dialogue_flag_dispatch.yaml"),
        ));
        assert_eq!(
            engine.handle_input("take iron key"),
            vec![Event::Took {
                object_id: ObjectId::new("iron-key"),
                object: "iron key".to_string(),
            }]
        );
        enter_corridor(&mut engine);
        // Flag not set → stock examine fires
        assert_eq!(
            engine.handle_input("examine iron key"),
            vec![Event::Examined {
                target: Target::Object(ObjectId::new("iron-key")),
                target_name: "iron key".to_string(),
            }]
        );
        // Talk to guard, choose the option that sets the flag
        engine.handle_input("talk to guard");
        engine.handle_input("choose 1");
        assert!(engine.world().has_flag("guard-told-about-key"));
        // Now the data interaction fires
        assert_eq!(
            engine.handle_input("examine iron key"),
            vec![Event::Custom {
                name: "key-glint".to_string()
            }]
        );
    }

    #[test]
    fn dialogue_flag_visible_in_query() {
        let mut engine = GameEngine::get(&world_with_npc_and_interactions(
            include_str!("../data/npcs_effects.yaml"),
            include_str!("../data/interactions/dialogue_flag_query.yaml"),
        ));
        assert_eq!(
            engine.handle_input("take iron key"),
            vec![Event::Took {
                object_id: ObjectId::new("iron-key"),
                object: "iron key".to_string(),
            }]
        );
        enter_corridor(&mut engine);
        // Flag not set → query returns nothing
        assert!(
            engine
                .interactions_for(Some(ObjectId::new("iron-key")), None)
                .is_empty()
        );
        // Talk, set the flag
        engine.handle_input("talk to guard");
        engine.handle_input("choose 1");
        // Query now returns the interaction
        assert_eq!(
            engine
                .interactions_for(None, Some(Target::Object(ObjectId::new("iron-key"))))
                .len(),
            1
        );
    }
}

// ---------------------------------------------------------------------------
// Query: NPCs as part of interactions_for
// ---------------------------------------------------------------------------

/// NPCs live in the open `interactions_for(None, None)` query as `Talk`
/// hotspots, so a point-and-click front-end renders every clickable thing in
/// one listing — NPCs and objects alike — without coercing NPCs into objects.
mod interactions_for_talk {
    use super::*;
    use core::{Target, TargetFilter, Verb};

    #[test]
    fn npc_in_current_room_is_a_talk_target() {
        let mut engine = engine_with(include_str!("../data/npcs_guard.yaml"));
        // Starts in the cellar; the guard lives in the corridor.
        assert!(engine.interactions_for(None, None).is_empty());

        enter_corridor(&mut engine);
        let listed = engine.interactions_for(None, None);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].verb(), Verb::Talk);
        assert_eq!(listed[0].npc(), Some(&NpcId::new("guard")));
        // An NPC hotspot is not an object verb: no item, any target.
        assert_eq!(listed[0].item(), None);
        assert_eq!(listed[0].target(), TargetFilter::Any);
    }

    #[test]
    fn leaving_the_room_removes_talk_targets() {
        let mut engine = engine_with(include_str!("../data/npcs_guard.yaml"));
        enter_corridor(&mut engine);
        assert_eq!(engine.interactions_for(None, None).len(), 1);

        engine.handle_input("go south");
        assert!(engine.interactions_for(None, None).is_empty());
    }

    #[test]
    fn every_npc_in_the_room_is_listed() {
        let mut engine = engine_with(include_str!("../data/npcs_two_npcs.yaml"));
        enter_corridor(&mut engine);
        let listed = engine.interactions_for(None, None);
        assert_eq!(listed.len(), 2);
        assert!(
            listed
                .iter()
                .any(|interaction| interaction.npc() == Some(&NpcId::new("guard")))
        );
        assert!(
            listed
                .iter()
                .any(|interaction| interaction.npc() == Some(&NpcId::new("warden")))
        );
    }

    #[test]
    fn targeted_queries_never_list_npc_hotspots() {
        let mut engine = engine_with(include_str!("../data/npcs_guard.yaml"));
        enter_corridor(&mut engine);
        let with_item = engine.interactions_for(Some(ObjectId::new("iron-key")), None);
        assert!(
            with_item
                .iter()
                .all(|interaction| interaction.verb() != Verb::Talk)
        );
        let with_target = engine.interactions_for(
            Some(ObjectId::new("iron-key")),
            Some(Target::Object(ObjectId::new("oak-door"))),
        );
        assert!(
            with_target
                .iter()
                .all(|interaction| interaction.verb() != Verb::Talk)
        );
    }

    #[test]
    fn talk_target_still_dispatches_via_talk_action() {
        let mut engine = engine_with(include_str!("../data/npcs_effects.yaml"));
        enter_corridor(&mut engine);
        // The open query surfaces the NPC as a talk hotspot...
        assert_eq!(engine.interactions_for(None, None).len(), 1);
        // ...while the text front-end still routes talk through `on_talk`.
        let events = engine.handle_input("talk to guard");
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], Event::Talked { npc, .. } if npc == "guard"));
    }
}

// ---------------------------------------------------------------------------
// Use item on NPC (roadmap: NPCs as first-class `Use` targets)
// ---------------------------------------------------------------------------

/// "Use X on guard" resolves the target as an NPC (npc-first over objects),
/// dispatches an interaction keyed to `target: npc: <key>`, and falls back to
/// the generic `Used` event (its `target_id` widened to a `Target::Npc`) when
/// nothing is authored.
///
/// `interactions_for` accepts `Target::Npc(...)` so a point-and-click UI can
/// offer NPCs as drop-targets.
mod use_on_npc {
    use super::*;
    use core::{Target, Verb};

    #[test]
    fn use_item_on_npc_without_interaction_emits_used() {
        let mut engine = engine_with(include_str!("../data/npcs_guard.yaml"));
        assert_eq!(
            engine.handle_input("take iron key"),
            vec![Event::Took {
                object_id: ObjectId::new("iron-key"),
                object: "iron key".to_string(),
            }]
        );
        enter_corridor(&mut engine);
        // The NPC carries its typed identity through the same `Used` event an
        // object target would (the `target_id` channel widened to `Target`).
        assert_eq!(
            engine.handle_input("use iron key on guard"),
            vec![Event::Used {
                object_id: ObjectId::new("iron-key"),
                object: "iron key".to_string(),
                target_id: Some(Target::Npc(NpcId::new("guard"))),
                target: Some("guard".to_string()),
            }]
        );
    }

    #[test]
    fn use_item_on_npc_runs_authored_interaction() {
        let mut engine = GameEngine::get(&world_with_npc_and_interactions(
            include_str!("../data/npcs_guard.yaml"),
            include_str!("../data/interactions/use_npc.yaml"),
        ));
        assert_eq!(
            engine.handle_input("take iron key"),
            vec![Event::Took {
                object_id: ObjectId::new("iron-key"),
                object: "iron key".to_string(),
            }]
        );
        enter_corridor(&mut engine);
        assert_eq!(
            engine.handle_input("use iron key on guard"),
            vec![Event::Custom {
                name: "key-shown-to-guard".to_string(),
            }]
        );
        // The authored effect is immutable state: a second use re-fires it.
        assert_eq!(
            engine.handle_input("use iron key on guard"),
            vec![Event::Custom {
                name: "key-shown-to-guard".to_string(),
            }]
        );
    }

    #[test]
    fn query_reports_npc_targeted_use_interaction() {
        let mut engine = GameEngine::get(&world_with_npc_and_interactions(
            include_str!("../data/npcs_guard.yaml"),
            include_str!("../data/interactions/use_npc.yaml"),
        ));
        enter_corridor(&mut engine);
        let listed = engine.interactions_for(
            Some(ObjectId::new("iron-key")),
            Some(Target::Npc(NpcId::new("guard"))),
        );
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].verb(), Verb::Use);
        assert_eq!(listed[0].item(), Some(ObjectId::new("iron-key")));
    }

    #[test]
    fn use_item_on_unknown_npc_reports_target_not_found() {
        let mut engine = engine_with(include_str!("../data/npcs_guard.yaml"));
        assert_eq!(
            engine.handle_input("take iron key"),
            vec![Event::Took {
                object_id: ObjectId::new("iron-key"),
                object: "iron key".to_string(),
            }]
        );
        enter_corridor(&mut engine);
        // No NPC (and no object) named "warden" → the generic object not-found.
        assert_eq!(
            engine.handle_input("use iron key on warden"),
            vec![Event::UsedTargetNotFound {
                object_id: ObjectId::new("iron-key"),
                object: "iron key".to_string(),
                target: "warden".to_string(),
            }]
        );
    }
}
