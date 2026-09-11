use std::collections::HashMap;

use crate::data::interactions_data::DataEffect;
use crate::keys::dialogue_node_id::DialogueNodeId;
use crate::keys::dialogue_option_id::DialogueOptionId;
use crate::{DialogueChoiceData, DialogueNodeData};

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
        let next = if data.next == DialogueNodeId::new(".end") {
            None
        } else {
            Some(data.next.clone())
        };

        DialogueChoice {
            option_id: data.option_id.clone(),
            label: data.label.clone(),
            next,
            effect: data.effect.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::dialogue_option_id::DialogueOptionId;

    #[test]
    fn find_choice_matches_a_one_based_index() {
        let node = DialogueNode {
            text: "Choose".to_string(),
            choices: vec![
                DialogueChoice {
                    option_id: None,
                    label: "Yes".to_string(),
                    next: None,
                    effect: Vec::new(),
                },
                DialogueChoice {
                    option_id: None,
                    label: "No".to_string(),
                    next: None,
                    effect: Vec::new(),
                },
            ],
        };
        assert_eq!(
            node.find_choice("1").map(DialogueChoice::label),
            Some("Yes")
        );
        assert_eq!(node.find_choice("2").map(DialogueChoice::label), Some("No"));
        assert!(node.find_choice("0").is_none());
        assert!(node.find_choice("3").is_none());
    }

    #[test]
    fn find_choice_matches_a_case_insensitive_label() {
        let node = DialogueNode {
            text: "Choose".to_string(),
            choices: vec![DialogueChoice {
                option_id: None,
                label: "Ask about the exit".to_string(),
                next: None,
                effect: Vec::new(),
            }],
        };
        assert!(node.find_choice("ASK ABOUT THE EXIT").is_some());
        assert!(node.find_choice("nonsense").is_none());
    }

    #[test]
    fn find_choice_prefers_index_over_label_when_both_could_match() {
        let node = DialogueNode {
            text: "Choose".to_string(),
            choices: vec![
                DialogueChoice {
                    option_id: None,
                    label: "First".to_string(),
                    next: None,
                    effect: Vec::new(),
                },
                DialogueChoice {
                    option_id: None,
                    label: "1".to_string(),
                    next: None,
                    effect: Vec::new(),
                },
            ],
        };
        // "1" parses as an index first, selecting the first choice by
        // position rather than the choice literally labelled "1".
        assert_eq!(
            node.find_choice("1").map(DialogueChoice::label),
            Some("First")
        );
    }

    #[test]
    fn dialogue_choice_from_data_treats_dot_end_as_conversation_end() {
        let data = DialogueChoiceData {
            option_id: Some(DialogueOptionId::new("opt-1")),
            label: "Bye".to_string(),
            next: DialogueNodeId::new(".end"),
            effect: Vec::new(),
        };
        let choice = DialogueChoice::from_data(&data);
        assert_eq!(choice.next(), None);
        assert_eq!(choice.option_id(), Some(&DialogueOptionId::new("opt-1")));
        assert_eq!(choice.label(), "Bye");
    }

    #[test]
    fn dialogue_choice_from_data_keeps_a_real_next_node() {
        let data = DialogueChoiceData {
            option_id: None,
            label: "Continue".to_string(),
            next: DialogueNodeId::new("node-2"),
            effect: Vec::new(),
        };
        let choice = DialogueChoice::from_data(&data);
        assert_eq!(choice.next(), Some(&DialogueNodeId::new("node-2")));
    }

    #[test]
    fn dialogue_node_from_data_converts_all_choices() {
        let data = DialogueNodeData {
            text: "Hello".to_string(),
            choices: vec![
                DialogueChoiceData {
                    option_id: None,
                    label: "Hi".to_string(),
                    next: DialogueNodeId::new(".end"),
                    effect: Vec::new(),
                },
                DialogueChoiceData {
                    option_id: None,
                    label: "Bye".to_string(),
                    next: DialogueNodeId::new(".end"),
                    effect: Vec::new(),
                },
            ],
        };
        let node = DialogueNode::from_data(&data);
        assert_eq!(node.text(), "Hello");
        assert_eq!(node.choices().len(), 2);
    }
}
