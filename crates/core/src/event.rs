use crate::Npc;
use crate::input::direction::Direction;
use crate::interaction::Target;
use crate::model::dialogue_node_id::DialogueNodeId;
use crate::model::dialogue_option_id::DialogueOptionId;
use crate::model::npc_id::NpcId;
use crate::world::npc::DialogueChoice as NpcDialogueChoice;
use crate::world::object::ObjectId;

/// Structured result of executing an `Action` against the world.
///
/// A UI consumes these events and decides how to present them. The engine
/// never produces prose — that is the job of a `View`. Keeping this as a
/// typed enum is what lets a text UI, a GUI, or any other front-end share
/// the same game logic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
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

    UnlockedExit {
        direction: Direction,
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
    /// The object was granted into inventory, regardless of where (if
    /// anywhere) it was placed in the world.
    Granted {
        object_id: ObjectId,
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
    /// The object was removed from inventory without being placed anywhere.
    Discarded {
        object_id: ObjectId,
        object: String,
    },

    /// The player used one object, optionally on a target (object or NPC).
    Used {
        object_id: ObjectId,
        object: String,
        target_id: Option<Target>,
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
        target_ids: Vec<Target>,
        target: String,
    },
    /// An attempted interaction that makes no sense ("use the sword on the
    /// open door"): the entities resolved fine but the combination does not
    /// apply. The generic fallback answer.
    CannotUse {
        item: String,
        target: String,
    },

    Examined {
        target: Target,
        target_name: String,
    },
    ExaminedTargetNotFound {
        target: String,
    },
    ExaminedTargetAmbiguous {
        target_ids: Vec<Target>,
        target: String,
    },

    FlagSet {
        flag: String,
    },
    FlagCleared {
        flag: String,
    },

    // -- NPC / dialogue --
    /// The player initiated or advanced a dialogue with an NPC.
    Talked {
        npc_id: NpcId,
        npc: String,
        node_id: DialogueNodeId,
        text: String,
        choices: Vec<DialogueChoice>,
    },
    /// A dialogue conversation ended (reached the end marker or no choices).
    DialogueEnded {
        npc_id: NpcId,
        npc: String,
    },
    /// No NPC by that name was found in the current room.
    TalkNpcNotFound {
        npc: String,
    },
    /// The player's choice didn't match any available dialogue option.
    DialogueInvalidChoice {
        npc: String,
        choice: String,
    },
}

impl Event {
    /// Build a `Talked` event showing `node_id` of `npc`'s dialogue, or
    /// `None` if the node is not part of the graph.
    #[must_use]
    pub(crate) fn talked(npc: &Npc, node_id: &DialogueNodeId) -> Option<Event> {
        let node = npc.dialogue_node(node_id)?;
        Some(Event::Talked {
            npc_id: npc.id.clone(),
            npc: npc.primary_name().to_string(),
            node_id: node_id.clone(),
            text: node.text().to_string(),
            choices: node.choices().iter().map(Into::into).collect(),
        })
    }

    /// Build a `DialogueEnded` event for `npc`.
    #[must_use]
    pub(crate) fn dialogue_ended(npc: &Npc) -> Event {
        Event::DialogueEnded {
            npc_id: npc.id.clone(),
            npc: npc.primary_name().to_string(),
        }
    }
}

/// A player choice presented inside an [`Event::Talked`] event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogueChoice {
    /// The stable authored id of the choice, if one was declared.
    pub option_id: Option<DialogueOptionId>,
    pub label: String,
    /// The next node id, or `None` to terminate the conversation.
    pub next: Option<DialogueNodeId>,
}

/// Bridge from the runtime dialogue choice model to the event payload.
impl From<&NpcDialogueChoice> for DialogueChoice {
    fn from(choice: &NpcDialogueChoice) -> Self {
        DialogueChoice {
            option_id: choice.option_id().cloned(),
            label: choice.label().to_string(),
            next: choice.next().cloned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::room_id::RoomId;
    use crate::world::npc::DialogueGraph;
    use std::collections::HashMap;

    fn npc_with_node(node_id: &str, text: &str, choices: Vec<NpcDialogueChoice>) -> Npc {
        let mut dialogue: DialogueGraph = HashMap::new();
        dialogue.insert(
            DialogueNodeId::new(node_id),
            crate::world::npc::DialogueNode {
                text: text.to_string(),
                choices,
            },
        );
        Npc {
            id: NpcId::new("guard"),
            primary_name: "Guard".to_string(),
            aliases: Vec::new(),
            room: RoomId::new("corridor"),
            root: DialogueNodeId::new(node_id),
            dialogue,
        }
    }

    #[test]
    fn talked_builds_an_event_for_a_known_node() {
        let npc = npc_with_node("start", "Halt!", Vec::new());
        let event = Event::talked(&npc, &DialogueNodeId::new("start")).expect("node exists");
        assert_eq!(
            event,
            Event::Talked {
                npc_id: NpcId::new("guard"),
                npc: "Guard".to_string(),
                node_id: DialogueNodeId::new("start"),
                text: "Halt!".to_string(),
                choices: Vec::new(),
            }
        );
    }

    #[test]
    fn talked_returns_none_for_an_unknown_node() {
        let npc = npc_with_node("start", "Halt!", Vec::new());
        assert_eq!(Event::talked(&npc, &DialogueNodeId::new("missing")), None);
    }

    #[test]
    fn dialogue_ended_carries_the_npcs_identity() {
        let npc = npc_with_node("start", "Halt!", Vec::new());
        assert_eq!(
            Event::dialogue_ended(&npc),
            Event::DialogueEnded {
                npc_id: NpcId::new("guard"),
                npc: "Guard".to_string(),
            }
        );
    }
}
