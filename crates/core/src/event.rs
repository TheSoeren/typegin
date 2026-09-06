use crate::input::direction::Direction;
use crate::world::object::ObjectId;

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

    /// An opaque, game-authored beat, emitted by custom rules or a `Custom`
    /// interaction effect. Consumers that don't recognise `name` may ignore
    /// or render it generically.
    Custom {
        name: String,
    },

    /// The player looked around the current room; the world can be re-rendered.
    Looked,

    /// The player moved in a direction.
    Went(Direction),
    WentExitHidden(Direction),
    WentExitLocked(Direction),
    WentInvalidDirection(Direction),

    /// A locked exit was unlocked (typically by using a matching `gated_by`
    /// object on it).
    UnlockedExit {
        direction: Direction,
    },

    /// An attempted interaction that makes no sense ("use the sword on the
    /// open door"): the entities resolved fine but the combination does not
    /// apply. The generic fallback answer.
    CannotUse {
        item: String,
        target: String,
    },

    /// The player took an object into inventory.
    Took {
        object_id: ObjectId,
        object: String,
    },
    TookObjectNotFound {
        object: String,
    },
    TookObjectAmbiguous {
        object_ids: Vec<ObjectId>,
        object: String,
    },

    /// The player tried to take a scene object (furniture, a door, ...). Scene
    /// objects stay in the world; only `Item`s are portable.
    CantTake {
        object: String,
    },

    /// The player dropped an object from inventory.
    Dropped {
        object_id: ObjectId,
        object: String,
    },
    DroppedObjectNotFound {
        object: String,
    },
    DroppedObjectAmbiguous {
        object_ids: Vec<ObjectId>,
        object: String,
    },

    /// The player used one object, optionally on a target.
    Used {
        object_id: ObjectId,
        object: String,
        target_id: Option<ObjectId>,
        target: Option<String>,
    },
    UsedObjectNotFound {
        object: String,
    },
    UsedObjectAmbiguous {
        object_ids: Vec<ObjectId>,
        object: String,
    },
    UsedTargetNeeded {
        object_id: ObjectId,
        object: String,
    },
    UsedTargetNotFound {
        object_id: ObjectId,
        object: String,
        target: String,
    },
    UsedTargetAmbiguous {
        object_id: ObjectId,
        object: String,
        target_ids: Vec<ObjectId>,
        target: String,
    },

    /// The player examined an object
    Examined {
        object_id: ObjectId,
        object: String,
    },
    ExaminedObjectNotFound {
        object: String,
    },
    ExaminedObjectAmbiguous {
        object_ids: Vec<ObjectId>,
        object: String,
    },
}
