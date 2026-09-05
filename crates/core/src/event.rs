use crate::{ItemId, input::Direction};

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
    WentExitHidden(Direction),
    WentExitLocked(Direction),
    WentInvalidDirection(Direction),

    /// The player took an item into inventory.
    Took {
        item_id: ItemId,
        item: String,
    },
    TookItemNotFound {
        item: String,
    },
    TookItemAmbiguous {
        item_ids: Vec<ItemId>,
        item: String,
    },

    /// The player dropped an item from inventory.
    Dropped {
        item_id: ItemId,
        item: String,
    },
    DroppedItemNotFound {
        item: String,
    },
    DroppedItemAmbiguous {
        item_ids: Vec<ItemId>,
        item: String,
    },

    /// The player used one item, optionally on a target.
    Used {
        item_id: ItemId,
        item: String,
        target_id: Option<ItemId>,
        target: Option<String>,
    },
    UsedItemNotFound {
        item: String,
    },
    UsedItemAmbiguous {
        item_ids: Vec<ItemId>,
        item: String,
    },
    UsedTargetNeeded {
        item_id: ItemId,
        item: String,
    },
    UsedTargetNotFound {
        item_id: ItemId,
        item: String,
        target: String,
    },
    UsedTargetAmbiguous {
        item_id: ItemId,
        item: String,
        target_ids: Vec<ItemId>,
        target: String,
    },

    /// The player examined an item
    Examined {
        item_id: ItemId,
        item: String,
    },
    ExaminedItemNotFound {
        item: String,
    },
    ExaminedItemAmbiguous {
        item_ids: Vec<ItemId>,
        item: String,
    },
}
