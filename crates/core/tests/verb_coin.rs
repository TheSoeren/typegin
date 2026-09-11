//! Spec for the verb-coin UI primitives: `GameEngine::verbs_for` and
//! `Rules::default_verbs`. A point-and-click front-end (Deponia-style: a
//! static per-hotspot verb coin, not a menu that grows and shrinks as
//! inventory changes) needs one query — "what verbs are possible for this
//! target, on its own" — to populate that coin.
//!
//! No new YAML schema, `Action`, or `Event`: this is a pure read-only query
//! layered on the existing `interactions_for`/dispatch machinery, so (unlike
//! `combine.rs`) there is no `parse` module here.
//!
//! ## New API
//!
//! * `GameEngine::verbs_for(&self, target: Target) -> HashSet<Verb>` — the union
//!   of every `Verb` a currently-live *item-agnostic* interaction reports for
//!   `target` (`interactions_for(None, Some(target))` — including the
//!   NPC-hotspot's `Verb::Talk` when `target` names a present NPC), unioned
//!   with `Rules::default_verbs`, deduplicated.
//!
//!   An interaction that requires a specific carried item (`item: <id>`)
//!   never contributes here, no matter what the player is holding — whether
//!   combining an item with `target` does anything is discovered separately,
//!   at the moment a specific item is actually used against it
//!   (`interactions_for(Some(item), Some(target))`, e.g. dragging an
//!   inventory item onto the hotspot), never by browsing the coin. This
//!   keeps the coin's contents a pure function of the *target and world
//!   state*: a hotspot's available verbs never change just because the
//!   player happened to pick something up, matching how the "reveal
//!   hotspots" affordance and the verb coin behave in modern point-and-click
//!   UIs (Deponia included) — both are static per room state, never
//!   inventory-reactive.
//! * `Rules::default_verbs(&self, target: Target, world: &WorldState) ->
//!   Vec<Verb>` — verbs that work on `target` unconditionally, entirely the
//!   consumer's call (a game might return all its verbs for every hotspot;
//!   another might return only `Examine`). The trait default returns
//!   `vec![Verb::Examine]` unconditionally (see `crates/core/src/rules/mod.rs`),
//!   inherited by `BasicRules` as-is, so `verbs_for` is never empty under
//!   `BasicRules` even with zero authored interactions. Exercised only
//!   through `verbs_for` here — `default_rules.rs`'s own convention is to
//!   drive every hook through the public API, never call it directly, and
//!   this suite follows suit rather than adding a one-off exception.
//!
//! Run with: `cargo test --test verb_coin`.

mod common;

use std::collections::HashSet;

use common::{
    base_world, engine_with_interactions as engine_with, engine_with_npcs, enter_corridor,
    world_with_interactions as world_with, world_with_npcs,
};
use core::{
    GameEngine, Interaction, NpcId, ObjectId, Rules, Target, TargetFilter, TargetKind, Verb,
    WorldState,
};

/// Minimal NPC (a guard in the corridor) with a single dialogue node.
const VERB_COIN_GUARD_YAML: &str = r#"npcs:
  - key: guard
    primary_name: guard
    aliases: [sentry]
    room: corridor
    dialogue:
      root: greeting
      nodes:
        greeting:
          text: "The guard nods at you.""#;

// ---------------------------------------------------------------------------
// Aggregation across authored data-driven interactions
// ---------------------------------------------------------------------------

mod data_driven_aggregation {
    use super::*;

    #[test]
    fn no_interactions_still_includes_basic_rules_default_verbs() {
        // `BasicRules` inherits `Rules::default_verbs`'s trait default
        // unmodified, which unconditionally returns `[Verb::Examine]` (see
        // `crates/core/src/rules/mod.rs`) — so even with zero authored
        // interactions, `verbs_for` is never empty under `BasicRules`.
        let engine = GameEngine::get(&base_world());
        assert_eq!(
            engine.verbs_for(Target::Object(ObjectId::new("rusty-lamp"))),
            HashSet::from([Verb::Examine])
        );
    }

    #[test]
    fn includes_an_item_agnostic_interaction() {
        let engine = engine_with(
            r"- verb: examine
  target:
    object: rusty-lamp",
        );
        assert!(
            engine
                .verbs_for(Target::Object(ObjectId::new("rusty-lamp")))
                .contains(&Verb::Examine)
        );
    }

    #[test]
    fn excludes_an_item_gated_interaction_even_once_that_item_is_carried() {
        let mut engine = engine_with(
            r"- verb: examine
  target:
    object: rusty-lamp
- verb: use
  item: brass-key
  target:
    object: rusty-lamp",
        );
        let target = Target::Object(ObjectId::new("rusty-lamp"));

        // Before picking anything up: only the item-agnostic Examine shows.
        let before = engine.verbs_for(target.clone());
        assert!(before.contains(&Verb::Examine));
        assert!(!before.contains(&Verb::Use));

        // Picking up the exact item the recipe needs must not change the
        // coin: combining is discovered by using the item on the target
        // directly, never by browsing the coin.
        engine.handle_input("take brass key");
        let after = engine.verbs_for(target);
        assert!(after.contains(&Verb::Examine));
        assert!(!after.contains(&Verb::Use));
    }

    #[test]
    fn dedups_a_verb_matched_by_multiple_item_agnostic_interactions() {
        let engine = engine_with(
            r"- verb: use
  target:
    object: rusty-lamp
  effect:
    - emit: first
- verb: use
  target:
    object: rusty-lamp
  effect:
    - emit: second",
        );

        let verbs = engine.verbs_for(Target::Object(ObjectId::new("rusty-lamp")));
        assert_eq!(verbs.iter().filter(|verb| **verb == Verb::Use).count(), 1);
    }
}

// ---------------------------------------------------------------------------
// Aggregation across closure-based `Rules::interactions`
// ---------------------------------------------------------------------------

mod closure_driven_aggregation {
    use super::*;

    struct AuthoredInteractions(Vec<Interaction>);
    impl Rules for AuthoredInteractions {
        fn interactions(&self) -> &[Interaction] {
            &self.0
        }
    }

    #[test]
    fn includes_verbs_from_rules_interactions() {
        let interaction = Interaction::build(
            Verb::Examine,
            None,
            TargetFilter::Kind(TargetKind::Scene),
            None,
            Box::new(|_world, _context| Vec::new()),
        );
        let engine =
            GameEngine::get_with_rules(&base_world(), AuthoredInteractions(vec![interaction]));

        assert!(
            engine
                .verbs_for(Target::Object(ObjectId::new("cellar-stairs")))
                .contains(&Verb::Examine)
        );
    }
}

// ---------------------------------------------------------------------------
// `Rules::default_verbs` — the consumer's own "works everywhere" policy
// ---------------------------------------------------------------------------

mod default_verbs_hook {
    use super::*;

    struct AllVerbsEverywhere;
    impl Rules for AllVerbsEverywhere {
        fn default_verbs(&self, _target: Target, _world: &WorldState) -> Vec<Verb> {
            vec![Verb::Examine, Verb::Look]
        }
    }

    #[test]
    fn custom_default_verbs_are_unioned_into_verbs_for() {
        let engine = GameEngine::get_with_rules(&base_world(), AllVerbsEverywhere);
        let verbs = engine.verbs_for(Target::Object(ObjectId::new("cellar-stairs")));
        assert!(verbs.contains(&Verb::Examine));
        assert!(verbs.contains(&Verb::Look));
    }

    struct SceneAwareDefaults;
    impl Rules for SceneAwareDefaults {
        fn default_verbs(&self, target: Target, world: &WorldState) -> Vec<Verb> {
            match target {
                Target::Object(id) if world.object_is_scene(&id) => vec![Verb::Examine],
                Target::Object(_) => vec![Verb::Take],
                Target::Npc(_) => Vec::new(),
            }
        }
    }

    #[test]
    fn default_verbs_receives_the_queried_target_and_world_state() {
        let engine = GameEngine::get_with_rules(&base_world(), SceneAwareDefaults);

        let scene_verbs = engine.verbs_for(Target::Object(ObjectId::new("cellar-stairs")));
        assert!(scene_verbs.contains(&Verb::Examine));
        assert!(!scene_verbs.contains(&Verb::Take));

        let item_verbs = engine.verbs_for(Target::Object(ObjectId::new("rusty-lamp")));
        assert!(item_verbs.contains(&Verb::Take));
        assert!(!item_verbs.contains(&Verb::Examine));
    }

    struct UseAndTalkEverywhere;
    impl Rules for UseAndTalkEverywhere {
        fn default_verbs(&self, _target: Target, _world: &WorldState) -> Vec<Verb> {
            vec![Verb::Use, Verb::Talk]
        }
    }

    #[test]
    fn custom_default_verbs_union_with_authored_verbs_without_duplication() {
        let engine = GameEngine::get_with_rules(
            &world_with(
                r"- verb: use
  target:
    object: rusty-lamp",
            ),
            UseAndTalkEverywhere,
        );

        let verbs = engine.verbs_for(Target::Object(ObjectId::new("rusty-lamp")));
        assert_eq!(verbs.iter().filter(|verb| **verb == Verb::Use).count(), 1);
        assert!(verbs.contains(&Verb::Talk));
    }
}

// ---------------------------------------------------------------------------
// NPC targets: `Verb::Talk` flows through the same `interactions_for`-backed
// aggregation as any other verb, no special casing in `verbs_for` itself.
// ---------------------------------------------------------------------------

mod npc_targets {
    use super::*;

    #[test]
    fn includes_talk_for_an_npc_present_in_the_room() {
        let mut engine = engine_with_npcs(VERB_COIN_GUARD_YAML);
        enter_corridor(&mut engine);
        assert!(
            engine
                .verbs_for(Target::Npc(NpcId::new("guard")))
                .contains(&Verb::Talk)
        );
    }

    #[test]
    fn excludes_talk_once_the_npc_has_left_scope() {
        // The player starts in The Cellar; the guard lives in the corridor.
        // `BasicRules`'s inherited `default_verbs` still contributes
        // `Examine` unconditionally (see the equivalent note on
        // `no_interactions_still_includes_basic_rules_default_verbs`), so
        // the coin isn't empty — only `Talk` must be gone.
        let engine = engine_with_npcs(VERB_COIN_GUARD_YAML);
        assert!(
            !engine
                .verbs_for(Target::Npc(NpcId::new("guard")))
                .contains(&Verb::Talk)
        );
    }

    struct ExamineEverywhere;
    impl Rules for ExamineEverywhere {
        fn default_verbs(&self, _target: Target, _world: &WorldState) -> Vec<Verb> {
            vec![Verb::Examine]
        }
    }

    #[test]
    fn talk_for_npc_target_still_unions_default_verbs() {
        let mut engine =
            GameEngine::get_with_rules(&world_with_npcs(VERB_COIN_GUARD_YAML), ExamineEverywhere);
        enter_corridor(&mut engine);

        let verbs = engine.verbs_for(Target::Npc(NpcId::new("guard")));
        assert!(verbs.contains(&Verb::Talk));
        assert!(verbs.contains(&Verb::Examine));
    }

    #[test]
    fn object_target_never_includes_talk() {
        let mut engine = engine_with_npcs(VERB_COIN_GUARD_YAML);
        enter_corridor(&mut engine);
        assert!(
            !engine
                .verbs_for(Target::Object(ObjectId::new("rusty-lamp")))
                .contains(&Verb::Talk)
        );
    }
}

// ---------------------------------------------------------------------------
// `verbs_for` is a pure query: it never mutates the world.
// ---------------------------------------------------------------------------

mod read_only_query {
    use super::*;

    #[test]
    fn verbs_for_does_not_mutate_the_world() {
        let engine = engine_with(
            r"- verb: use
  target:
    object: rusty-lamp
  effect:
    - discard: rusty-lamp
    - grant: rusty-nail",
        );

        let target = Target::Object(ObjectId::new("rusty-lamp"));
        assert!(engine.verbs_for(target.clone()).contains(&Verb::Use));
        assert!(engine.verbs_for(target).contains(&Verb::Use));

        assert!(!engine.world().player_holds(&ObjectId::new("rusty-nail")));
    }
}
