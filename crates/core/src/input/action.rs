use serde::Deserialize;

use crate::{NpcId, ObjectId, Target};

use super::direction::Direction;

/// A parsed, structured player command, independent of the exact wording.
///
/// Produced by [`parse_input`](crate::parse_input) from a text line and passed
/// to [`GameEngine::execute_action`](crate::GameEngine::execute_action). A
/// front-end can also construct one directly (for example a GUI button that
/// maps to `Action::Go(Direction::North)`).
///
/// Every targeted verb carries a [`Locator`] rather than a bare name: a text
/// parser only ever has a typed noun to give (`Locator::Name`), resolved
/// against the world at execution time with full found/ambiguous/not-found
/// semantics; a point-and-click front-end that already knows the concrete id
/// (from [`GameEngine::interactions_for`](crate::GameEngine::interactions_for)/
/// [`GameEngine::verbs_for`](crate::GameEngine::verbs_for)) can supply it
/// directly (`Locator::Id`) and skip name resolution - and the ambiguity
/// that makes sense for typed text but not for a click that already picked
/// exactly one thing.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Action {
    /// The player looked around the current room.
    Look,
    /// The player moved towards `direction`, or entered a named/clicked exit.
    Go(GoTarget),
    /// The player examined the thing referenced by the locator.
    Examine(Locator<Target>),
    /// The player tried to pick up the item referenced by the locator.
    Take(Locator<ObjectId>),
    /// The player tried to drop the item referenced by the locator.
    Drop(Locator<ObjectId>),
    /// The player used an item, optionally on a target.
    Use {
        item: Locator<ObjectId>,
        target: Option<Locator<Target>>,
    },
    /// The player initiated a dialogue with an NPC.
    Talk(Locator<NpcId>),
    /// The player chose a dialogue option (by index or label).
    Choose(String),
    /// A command that matched no known action.
    Unknown(String),
}

/// A reference to something an [`Action`] targets: the raw name a text
/// parser typed, or a concrete id a caller already knows. See [`Action`]'s
/// doc comment for why both exist.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Locator<Id> {
    /// A player-typed (or point-and-click "enter") noun, resolved against the
    /// world at execution time.
    Name(String),
    /// An id the caller already knows is the intended target, resolved
    /// directly with no name matching (and no ambiguity).
    Id(Id),
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, Deserialize)]
#[serde(untagged)]
pub enum GoTarget {
    Direction(Direction),
    Named(String),
    /// An exit's door object, referenced directly by id (the point-and-click
    /// counterpart to `Named`). Never produced by YAML content - an author
    /// only ever has a symbolic name to write, never a runtime id - so this
    /// variant is unreachable from `#[serde(untagged)]` deserialization; it
    /// exists purely for a front-end to construct an `Action` directly.
    Id(ObjectId),
}

/// Outcome of a world mutation attempt: taking, granting, discarding, or
/// dropping an object, or moving the player to another room. Every such
/// attempt is a plain success-or-no-op - none carries payload data beyond
/// that - so one shared type stands in for what were five identically-shaped
/// per-operation enums.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Success,
    Fail,
}
