//! Spec for the verb-coin UI primitives: `GameEngine::verbs_for` and
//! `Rules::verbs_for`. A point-and-click front-end (Deponia-style: a
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
//! * `GameEngine::verbs_for(&self, target: Target) -> HashSet<Verb>` — computes
//!   every `Verb` a currently-live *item-agnostic* interaction reports for
//!   `target` (`interactions_for(None, Some(target))` — including the
//!   NPC-hotspot's `Verb::Talk` when `target` names a present NPC), then hands
//!   that off to `Rules::verbs_for`, which has the final say over what the
//!   coin actually holds.
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
//! * `Rules::verbs_for(&self, target: Target, interaction_verbs: &HashSet<Verb>,
//!   world: &WorldState) -> HashSet<Verb>` — decides the coin's final
//!   contents, entirely the consumer's call: whatever this returns *is* the
//!   result, so an override that wants to keep `interaction_verbs` around has
//!   to fold them back in itself (see `default_verbs_hook`'s
//!   `AllVerbsEverywhere` for one that doesn't, and `door_targets`'s
//!   `MergeInteractionsOntoExits` for one that does). The trait default
//!   (see `crates/core/src/rules/mod.rs`) folds `interaction_verbs` in plus
//!   `Verb::Examine` — except for an open (unlocked) exit, where it
//!   deliberately discards `interaction_verbs` and returns `Verb::Go` alone.
//!   Inherited by `BasicRules` as-is, so `verbs_for` is never empty under
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
use core::{BasicRules, GameEngine, Interaction, Rules, Target, TargetFilter, Verb, WorldState};

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
        // `BasicRules` inherits `Rules::verbs_for`'s trait default
        // unmodified, which always offers `Examine` and mirrors what the
        // stock `on_take`/`on_drop` hooks would actually do (see
        // `crates/core/src/rules/mod.rs`) — `rusty-lamp` is a non-`Scene`
        // item currently in the room, so `on_take` would succeed on it,
        // and `verbs_for` reflects that with `Take` alongside `Examine`,
        // even with zero authored interactions.
        let engine = GameEngine::get(&base_world());
        assert_eq!(
            engine.verbs_for(Target::Object(core::objectId!("rusty-lamp"))),
            HashSet::from([Verb::Examine, Verb::Take])
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
                .verbs_for(Target::Object(core::objectId!("rusty-lamp")))
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
        let target = Target::Object(core::objectId!("rusty-lamp"));

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

        let verbs = engine.verbs_for(Target::Object(core::objectId!("rusty-lamp")));
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
        // Targets a plain item, not a door: this test is about closure
        // interactions surfacing through `verbs_for` at all, orthogonal to
        // the door-exclusivity behaviour covered by `door_targets` below.
        let interaction = Interaction::build(
            Verb::Examine,
            None,
            TargetFilter::Targeted,
            None,
            Box::new(|_world, _context| Vec::new()),
        );
        let engine =
            GameEngine::get_with_rules(&base_world(), AuthoredInteractions(vec![interaction]));

        assert!(
            engine
                .verbs_for(Target::Object(core::objectId!("iron-key")))
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
        fn verbs_for(
            &self,
            _target: Target,
            _interaction_verbs: &HashSet<Verb>,
            _world: &WorldState,
        ) -> HashSet<Verb> {
            HashSet::from([Verb::Examine, Verb::Look])
        }
    }

    #[test]
    fn custom_default_verbs_are_unioned_into_verbs_for() {
        let engine = GameEngine::get_with_rules(&base_world(), AllVerbsEverywhere);
        let verbs = engine.verbs_for(Target::Object(core::objectId!("cellar-stairs")));
        assert!(verbs.contains(&Verb::Examine));
        assert!(verbs.contains(&Verb::Look));
    }

    struct SceneAwareDefaults;
    impl Rules for SceneAwareDefaults {
        fn verbs_for(
            &self,
            target: Target,
            _interaction_verbs: &HashSet<Verb>,
            world: &WorldState,
        ) -> HashSet<Verb> {
            match target {
                Target::Object(id) if world.object_is_scene(&id) => HashSet::from([Verb::Examine]),
                Target::Object(_) => HashSet::from([Verb::Take]),
                Target::Npc(_) => HashSet::new(),
            }
        }
    }

    #[test]
    fn default_verbs_receives_the_queried_target_and_world_state() {
        let engine = GameEngine::get_with_rules(&base_world(), SceneAwareDefaults);

        let scene_verbs = engine.verbs_for(Target::Object(core::objectId!("cellar-stairs")));
        assert!(scene_verbs.contains(&Verb::Examine));
        assert!(!scene_verbs.contains(&Verb::Take));

        let item_verbs = engine.verbs_for(Target::Object(core::objectId!("rusty-lamp")));
        assert!(item_verbs.contains(&Verb::Take));
        assert!(!item_verbs.contains(&Verb::Examine));
    }

    struct UseAndTalkEverywhere;
    impl Rules for UseAndTalkEverywhere {
        fn verbs_for(
            &self,
            _target: Target,
            _interaction_verbs: &HashSet<Verb>,
            _world: &WorldState,
        ) -> HashSet<Verb> {
            HashSet::from([Verb::Use, Verb::Talk])
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

        let verbs = engine.verbs_for(Target::Object(core::objectId!("rusty-lamp")));
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
                .verbs_for(Target::Npc(core::npcId!("guard")))
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
                .verbs_for(Target::Npc(core::npcId!("guard")))
                .contains(&Verb::Talk)
        );
    }

    /// An override always offers `Examine`, but — unlike the stock
    /// default — has to fold `interaction_verbs` back in itself if it wants
    /// them to survive: `Rules::verbs_for` has the final say, so an override
    /// that ignores `interaction_verbs` loses them, `Talk` included.
    struct ExamineEverywhere;
    impl Rules for ExamineEverywhere {
        fn verbs_for(
            &self,
            _target: Target,
            interaction_verbs: &HashSet<Verb>,
            _world: &WorldState,
        ) -> HashSet<Verb> {
            let mut verbs = interaction_verbs.clone();
            verbs.insert(Verb::Examine);
            verbs
        }
    }

    #[test]
    fn talk_for_npc_target_still_unions_default_verbs() {
        let mut engine =
            GameEngine::get_with_rules(&world_with_npcs(VERB_COIN_GUARD_YAML), ExamineEverywhere);
        enter_corridor(&mut engine);

        let verbs = engine.verbs_for(Target::Npc(core::npcId!("guard")));
        assert!(verbs.contains(&Verb::Talk));
        assert!(verbs.contains(&Verb::Examine));
    }

    #[test]
    fn object_target_never_includes_talk() {
        let mut engine = engine_with_npcs(VERB_COIN_GUARD_YAML);
        enter_corridor(&mut engine);
        assert!(
            !engine
                .verbs_for(Target::Object(core::objectId!("rusty-lamp")))
                .contains(&Verb::Talk)
        );
    }
}

// ---------------------------------------------------------------------------
// Door targets: an *open* exit collapses to a single `Verb::Go` affordance
// by default -- no verb-coin at all, matching the "just a walk cursor"
// convention modern point-and-click adventures use for an exit that needs
// no further interaction. A *locked* exit falls back to the ordinary
// default (`Examine`) instead, since the player can't walk through it yet --
// `Go` isn't offered until it's actually unlocked. The open-exit case is
// exclusive on purpose: `Rules::verbs_for` has the final say over what the
// coin holds, and the stock default discards `interaction_verbs` entirely
// once it decides an exit is open, rather than merging them in. A consumer
// who wants an authored interaction to still show up alongside `Go` has to
// override `verbs_for` and union `interaction_verbs` back in itself -- see
// `an_authored_interaction_can_still_add_a_verb_to_an_unlocked_door`.
// ---------------------------------------------------------------------------

mod door_targets {
    use super::*;
    use common::enter_study;

    #[test]
    fn an_unlocked_door_offers_only_go_by_default() {
        let mut engine = GameEngine::get(&base_world());
        enter_study(&mut engine);
        assert_eq!(
            engine.verbs_for(Target::Object(core::objectId!("wooden-door"))),
            HashSet::from([Verb::Go])
        );
    }

    #[test]
    fn a_locked_door_falls_back_to_the_ordinary_default() {
        let mut engine = GameEngine::get(&base_world());
        enter_study(&mut engine);
        let verbs = engine.verbs_for(Target::Object(core::objectId!("oak-door")));
        assert_eq!(verbs, HashSet::from([Verb::Examine]));
        assert!(!verbs.contains(&Verb::Go));
    }

    #[test]
    fn unlocking_a_door_switches_its_default_verb_from_examine_to_go() {
        let mut engine = engine_with(
            r"- verb: use
  item: iron-key
  target:
    object: oak-door
  effect:
    - unlock_exit: east",
        );
        engine.handle_input("take iron key");
        enter_study(&mut engine);
        let target = Target::Object(core::objectId!("oak-door"));

        assert_eq!(
            engine.verbs_for(target.clone()),
            HashSet::from([Verb::Examine])
        );
        engine.handle_input("use iron key on oak door");
        assert_eq!(engine.verbs_for(target), HashSet::from([Verb::Go]));
    }

    #[test]
    fn an_authored_interaction_can_still_add_a_verb_to_an_unlocked_door() {
        // The stock default would discard the authored `examine` interaction
        // here (an open exit collapses to `Go` alone) -- getting both back
        // requires overriding `verbs_for` and unioning `interaction_verbs`
        // onto the stock behaviour explicitly, as this override does.
        struct MergeInteractionsOntoExits;
        impl Rules for MergeInteractionsOntoExits {
            fn verbs_for(
                &self,
                target: Target,
                interaction_verbs: &HashSet<Verb>,
                world: &WorldState,
            ) -> HashSet<Verb> {
                let mut verbs = BasicRules.verbs_for(target, interaction_verbs, world);
                verbs.extend(interaction_verbs.clone());
                verbs
            }
        }

        let data = world_with(
            r"- verb: examine
  target:
    object: wooden-door",
        );
        let mut engine = GameEngine::get_with_rules(&data, MergeInteractionsOntoExits);
        enter_study(&mut engine);
        let verbs = engine.verbs_for(Target::Object(core::objectId!("wooden-door")));
        assert!(verbs.contains(&Verb::Go));
        assert!(verbs.contains(&Verb::Examine));
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

        let target = Target::Object(core::objectId!("rusty-lamp"));
        assert!(engine.verbs_for(target.clone()).contains(&Verb::Use));
        assert!(engine.verbs_for(target).contains(&Verb::Use));

        assert!(!engine.world().player_holds(&core::objectId!("rusty-nail")));
    }
}
