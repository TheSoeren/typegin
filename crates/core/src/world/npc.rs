mod dialogue;

use crate::data::npc_data::NpcData;
use crate::keys::dialogue_node_id::DialogueNodeId;
use crate::keys::room_id::RoomId;

pub use crate::keys::npc_id::NpcId;
pub use dialogue::{DialogueChoice, DialogueGraph, DialogueNode};

/// A live NPC in the world, built from [`NpcData`].
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

    /// The dialogue node with `id`, if it exists in this NPC's graph.
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
            .map(|(id, node)| (id.clone(), DialogueNode::from_data(node)))
            .collect();

        Npc {
            id: data.id.clone(),
            primary_name: data.primary_name.clone(),
            aliases: data.aliases.clone(),
            room: data.room.clone(),
            root: data.dialogue.root.clone(),
            dialogue: graph,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn npc(aliases: Vec<&str>) -> Npc {
        Npc {
            id: NpcId::new("guard"),
            primary_name: "guard".to_string(),
            aliases: aliases.into_iter().map(String::from).collect(),
            room: RoomId::new("corridor"),
            root: DialogueNodeId::new("start"),
            dialogue: DialogueGraph::new(),
        }
    }

    #[test]
    fn has_name_matches_primary_name_and_aliases() {
        let npc = npc(vec!["captain", "the guard"]);
        assert!(npc.has_name("guard"));
        assert!(npc.has_name("captain"));
        assert!(npc.has_name("the guard"));
        assert!(!npc.has_name("stranger"));
    }

    #[test]
    fn dialogue_node_looks_up_by_id_in_the_graph() {
        let mut npc = npc(vec![]);
        npc.dialogue.insert(
            DialogueNodeId::new("start"),
            DialogueNode {
                text: "Halt!".to_string(),
                choices: Vec::new(),
            },
        );
        assert_eq!(
            npc.dialogue_node(&DialogueNodeId::new("start"))
                .map(DialogueNode::text),
            Some("Halt!")
        );
        assert!(npc.dialogue_node(&DialogueNodeId::new("missing")).is_none());
    }
}
