//! Runtime dispatch for authored [`TriggerData`](crate::data::trigger_data::TriggerData)
//! (`data::trigger_data` is schema only, like every other `*_data` module):
//! checked after *every* player action, not tied to a specific verb — the
//! "World" hook the interaction system's verb-object dispatch doesn't cover.
//!
//! Dispatch contract (AGENTS.md engine gap #3, pinned down by
//! `crates/core/tests/triggers.rs`): every action re-checks every
//! not-yet-fired trigger; all whose conditions hold fire in one pass, in
//! declaration order, against a readiness snapshot taken before any of them
//! run in that pass — so one trigger's effect cannot make another trigger in
//! the same pass newly eligible (a chain resolves over multiple turns, never
//! within one). Each trigger fires at most once, ever.

use crate::data::interactions_data::DataEffect;
use crate::data::trigger_data::TriggerData;
use crate::event::Event;
use crate::interaction::ActionContext;
use crate::world::WorldState;

/// Check every not-yet-fired trigger against the current world state and run
/// whichever are ready, in declaration order, marking each fired.
#[must_use]
pub(crate) fn check_triggers(world: &mut WorldState) -> Vec<Event> {
    // Triggers carry no item/target of their own, so conditions are evaluated
    // against an empty context (this only matters for `DataCondition::IsDoor`,
    // which then always sees "no target present").
    let context = ActionContext::new(None, None, None);

    // Snapshot which triggers are ready *before* any of them run, so one
    // trigger's effect cannot make another trigger in this same pass newly
    // eligible.
    let ready: Vec<TriggerData> = world
        .triggers()
        .iter()
        .filter(|trigger| !world.trigger_fired(&trigger.id))
        .filter(|trigger| {
            trigger
                .condition
                .iter()
                .all(|condition| condition.matches(world, &context))
        })
        .cloned()
        .collect();

    let mut events = Vec::new();
    for trigger in ready {
        events.extend(DataEffect::apply_all(&trigger.effect, world));
        world.mark_trigger_fired(trigger.id.clone());
    }
    events
}
