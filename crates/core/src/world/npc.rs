use std::collections::HashMap;

use crate::data::interactions_data::DataEffect;
use crate::data::npc_data::NpcData;
use crate::model::dialogue_node_id::DialogueNodeId;
use crate::model::dialogue_option_id::DialogueOptionId;
use crate::model::room_id::RoomId;
use crate::{DialogueChoiceData, DialogueNodeData};

pub use crate::model::npc_id::NpcId;

/// A live NPC in the world, built from [`NpcData`](crate::data::npc_data::NpcData).
#[derive(Debug, Clone)]
pub struct Npc {
    pub(crate) id: NpcId,
    pub(crate) primary_name: String,
    pub(crate) aliases: Vec<String>,
    pub(crate) room: RoomId,
    /// The node a fresh conversation with this NPC starts at.
    pub(crate) root: DialogueNodeId,
    pub(crate) dialogue: DialogueGraph,
}

impl Npc {
    /// The NPC's unique id.
    #[must_use]
    pub fn id(&self) -> &NpcId {
        &self.id
    }

    /// The NPC's display name.
    #[must_use]
    pub fn primary_name(&self) -> &str {
        &self.primary_name
    }

    /// The NPC's alternative names.
    #[must_use]
    pub fn aliases(&self) -> &[String] {
        &self.aliases
    }

    /// The room this NPC lives in.
    #[must_use]
    pub fn room_id(&self) -> &RoomId {
        &self.room
    }

    /// The node a fresh conversation with this NPC starts at.
    #[must_use]
    pub fn root(&self) -> &DialogueNodeId {
        &self.root
    }

    /// The NPC's dialogue graph.
    #[must_use]
    pub fn dialogue(&self) -> &DialogueGraph {
        &self.dialogue
    }

    #[must_use]
    pub fn dialogue_node(&self, id: &DialogueNodeId) -> Option<&DialogueNode> {
        self.dialogue().get(id)
    }

    /// Whether `name` matches this NPC's primary name or any alias.
    #[must_use]
    pub fn has_name(&self, name: &str) -> bool {
        self.primary_name == name || self.aliases.iter().any(|alias| alias == name)
    }

    /// Build a live [`Npc`] from authored [`NpcData`].
    pub(crate) fn from_data(data: &NpcData) -> Self {
        let graph: DialogueGraph = data
            .dialogue
            .nodes
            .iter()
            .map(|(key, node)| {
                (
                    DialogueNodeId::new(key.clone()),
                    DialogueNode::from_data(node),
                )
            })
            .collect();

        Npc {
            id: data.id.clone(),
            primary_name: data.primary_name.clone(),
            aliases: data.aliases.clone(),
            room: data.room.clone(),
            root: DialogueNodeId::new(data.dialogue.root.clone()),
            dialogue: graph,
        }
    }
}

/// A dialogue graph: a map of node ids to dialogue nodes.
pub type DialogueGraph = HashMap<DialogueNodeId, DialogueNode>;

/// A single node in a dialogue graph: the NPC's line and the player's choices.
#[derive(Debug, Clone)]
pub struct DialogueNode {
    pub(crate) text: String,
    pub(crate) choices: Vec<DialogueChoice>,
}

impl DialogueNode {
    /// The NPC's spoken text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The player's available choices at this node.
    #[must_use]
    pub fn choices(&self) -> &[DialogueChoice] {
        &self.choices
    }

    /// Resolve a player's `choose` input to a choice: a 1-based index, or a
    /// case-insensitive match on the choice label.
    #[must_use]
    pub fn find_choice(&self, input: &str) -> Option<&DialogueChoice> {
        input
            .parse::<usize>()
            .ok()
            .and_then(|index| index.checked_sub(1))
            .and_then(|index| self.choices.get(index))
            .or_else(|| {
                self.choices()
                    .iter()
                    .find(|choice| choice.label().eq_ignore_ascii_case(input))
            })
    }

    pub(crate) fn from_data(data: &DialogueNodeData) -> Self {
        DialogueNode {
            text: data.text.clone(),
            choices: data.choices.iter().map(DialogueChoice::from_data).collect(),
        }
    }
}

/// A player choice within a dialogue node.
#[derive(Debug, Clone)]
pub struct DialogueChoice {
    /// Stable authored id for this choice, if the author declared one. Used to
    /// identify a choice across a run (e.g. by a point-and-click UI).
    pub(crate) option_id: Option<DialogueOptionId>,
    pub(crate) label: String,
    /// The next node id; `None` ends the conversation.
    pub(crate) next: Option<DialogueNodeId>,
    pub(crate) effect: Vec<DataEffect>,
}

impl DialogueChoice {
    /// The stable authored id of this choice, if declared.
    #[must_use]
    pub fn option_id(&self) -> Option<&DialogueOptionId> {
        self.option_id.as_ref()
    }

    /// The display label for this choice.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The next node id, or `None` to terminate the conversation.
    #[must_use]
    pub fn next(&self) -> Option<&DialogueNodeId> {
        self.next.as_ref()
    }

    /// Effects applied when this choice is selected.
    #[must_use]
    pub fn effect(&self) -> &[DataEffect] {
        &self.effect
    }

    pub(crate) fn from_data(data: &DialogueChoiceData) -> Self {
        let next = if data.next == ".end" {
            None
        } else {
            Some(DialogueNodeId::new(data.next.clone()))
        };

        DialogueChoice {
            option_id: data.option_id.clone(),
            label: data.label.clone(),
            next,
            effect: data.effect.clone(),
        }
    }
}
