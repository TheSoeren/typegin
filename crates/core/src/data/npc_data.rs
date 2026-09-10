use std::collections::HashMap;

use serde::Deserialize;

use crate::data::interactions_data::DataEffect;
use crate::data::{WorldData, WorldDataError};
use crate::model::dialogue_option_id::DialogueOptionId;
use crate::model::npc_id::NpcId;
use crate::model::room_id::RoomId;

/// A single NPC definition from world data (YAML).
///
/// Each NPC lives in a specific room and carries an inline dialogue graph.
#[derive(Debug, Clone, Deserialize)]
pub struct NpcData {
    #[serde(rename = "key")]
    pub id: NpcId,
    pub primary_name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub room: RoomId,
    pub dialogue: DialogueData,
}

/// The dialogue graph for an NPC: a root node id and a map of named nodes.
#[derive(Debug, Clone, Deserialize)]
pub struct DialogueData {
    pub root: String,
    #[serde(default)]
    pub nodes: HashMap<String, DialogueNodeData>,
}

/// A single node in a dialogue graph: the NPC's line and the player's choices.
#[derive(Debug, Clone, Deserialize)]
pub struct DialogueNodeData {
    pub text: String,
    #[serde(default)]
    pub choices: Vec<DialogueChoiceData>,
}

/// A player choice within a dialogue node: a stable id, a label, the next node
/// (or `.end` to terminate the conversation), and effects that run when the
/// choice is selected.
#[derive(Debug, Clone, Deserialize)]
pub struct DialogueChoiceData {
    #[serde(default, rename = "key")]
    pub option_id: Option<DialogueOptionId>,
    pub label: String,
    pub next: String,
    #[serde(default)]
    pub effect: Vec<DataEffect>,
}

/// The top-level shape of an `npcs.yaml` file.
#[derive(Debug, Deserialize)]
pub(crate) struct NpcsFile {
    #[serde(default)]
    pub(crate) npcs: Vec<NpcData>,
}

impl NpcData {
    /// Verify structure against the whole world data: the room exists and
    /// every dialogue node / choice target resolves within this NPC's graph.
    ///
    /// # Errors
    ///
    /// Returns a [`WorldDataError::Validation`] naming the first problem.
    pub(crate) fn validate_references(&self, data: &WorldData) -> Result<(), WorldDataError> {
        data.find_room(&self.room).ok_or_else(|| {
            WorldDataError::Validation(format!(
                "npc `{}` references unknown room id `{}`",
                self.id, self.room
            ))
        })?;

        if !self.dialogue.nodes.contains_key(&self.dialogue.root) {
            return Err(WorldDataError::Validation(format!(
                "npc `{}` dialogue root references unknown node `{}`",
                self.id, self.dialogue.root
            )));
        }

        for (node_id, node) in &self.dialogue.nodes {
            for choice in &node.choices {
                if choice.next != ".end" && !self.dialogue.nodes.contains_key(&choice.next) {
                    return Err(WorldDataError::Validation(format!(
                        "npc `{}` dialogue node `{}` choice references unknown node `{}`",
                        self.id, node_id, choice.next
                    )));
                }
                for effect in &choice.effect {
                    effect.validate_references(data)?;
                }
            }
        }

        Ok(())
    }
}
