use super::direction::Direction;

/// A parsed, structured player command, independent of the exact wording.
///
/// Produced by [`parse_input`](crate::parse_input) from a text line and passed
/// to [`GameEngine::execute_action`](crate::GameEngine::execute_action). A
/// front-end can also construct one directly (for example a GUI button that
/// maps to `Action::Go(Direction::North)`).
///
/// The string payloads are the verbatim nouns the player typed (e.g.
/// `"iron key"`); the engine resolves them against the world at execution time.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Action {
    /// The player looked around the current room.
    Look,
    /// The player moved towards `direction`.
    Go(Direction),
    /// The player examined the thing named by the string.
    Examine(String),
    /// The player tried to pick up the item named by the string.
    Take(String),
    /// The player tried to drop the item named by the string.
    Drop(String),
    /// The player used an item, optionally on a target (both by name).
    Use {
        item: String,
        target: Option<String>,
    },
    /// The player initiated a dialogue with an NPC.
    Talk(String),
    /// The player chose a dialogue option (by index or label).
    Choose(String),
    /// A command that matched no known action.
    Unknown(String),
}

/// Outcome of a world mutation attempt: taking, granting, discarding, or
/// dropping an object, or moving the player to another room. Every such
/// attempt is a plain success-or-no-op — none carries payload data beyond
/// that — so one shared type stands in for what were five identically-shaped
/// per-operation enums.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Success,
    Fail,
}
