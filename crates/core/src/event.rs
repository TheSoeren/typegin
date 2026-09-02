use crate::input::Direction;

/// Structured result of executing an `Action` against the world.
///
/// A UI consumes these events and decides how to present them. The engine
/// never produces prose — that is the job of a `View`. Keeping this as a
/// typed enum is what lets a text UI, a GUI, or any other front-end share
/// the same game logic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// The player typed an unknown command
    UnknownEvent {
        name: String,
    },

    /// The player looked around the current room; the world can be re-rendered.
    Looked,

    /// The player moved in a direction.
    Went(Direction),
    WentInvalidDirection(Direction),

    /// The player took an item into inventory.
    Took {
        item: String,
    },
    TookItemNotFound {
        item: String,
    },
    TookItemAmbiguous {
        item: String,
    },

    /// The player dropped an item from inventory.
    Dropped {
        item: String,
    },
    DroppedItemNotFound {
        item: String,
    },
    DroppedItemAmbiguous {
        item: String,
    },

    /// The player used one item, optionally on a target.
    Used {
        item: String,
        target: Option<String>,
    },
    UsedItemNotFound {
        item: String,
    },
    UsedItemAmbiguous {
        item: String,
    },
    UsedTargetNeeded {
        item: String,
    },
    UsedTargetNotFound {
        item: String,
        target: String,
    },
    UsedTargetAmbiguous {
        item: String,
    },

    /// The player examined an item
    Examined {
        item: String,
    },
    ExaminedItemNotFound {
        item: String,
    },
    ExaminedItemAmbiguous {
        item: String,
    },
}
