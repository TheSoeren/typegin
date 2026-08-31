use crate::input::Direction;

/// Structured result of executing an `Action` against the world.
///
/// A UI consumes these events and decides how to present them. The engine
/// never produces prose — that is the job of a `View`. Keeping this as a
/// typed enum is what lets a text UI, a GUI, or any other front-end share
/// the same game logic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// The player looked around the current room; the world can be re-rendered.
    Looked,

    /// The player moved in a direction.
    Went(Direction),

    /// The player took an item into inventory.
    Took { item: String },

    /// The player dropped an item from inventory.
    Dropped { item: String },

    /// The player used one item, optionally on a target.
    Used {
        item: String,
        target: Option<String>,
    },

    /// The player is already holding the item.
    AlreadyHolding { item: String },

    /// A named entity could not be matched to anything in the world.
    NotFound { phrase: String },

    /// The named entity matched more than one thing in the world.
    Ambiguous { phrase: String },

    /// A generic prose message, useful for custom behaviour and scripts.
    Message(String),
}
