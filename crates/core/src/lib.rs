//! Front-end-agnostic text-adventure engine: parsing, world state, and a
//! data-driven interaction system. The engine only ever hands a front-end a
//! typed [`Event`] stream and read-only [`WorldState`] queries — it has no
//! opinion on rendering (no `View` trait, no output type of any kind), so a
//! text UI, a GUI, or a full point-and-click front-end each build whatever
//! rendering layer suits them, entirely outside this crate.

pub mod data;
pub mod engine;
pub mod event;
pub mod input;
pub mod interaction;
pub mod keys;
pub mod rules;
pub mod trigger;
pub mod world;

pub use data::interactions_data::{DataCondition, DataEffect, DataTarget, InteractionData};
pub use data::npc_data::{DialogueChoiceData, DialogueData, DialogueNodeData, NpcData};
pub use data::object_data;
pub use data::trigger_data::TriggerData;
pub use data::{ExtraValue, WorldData, WorldDataError};
pub use engine::GameEngine;
pub use event::{DialogueChoice, Event};
pub use input::parse_input;
pub use input::{Action, Direction, GoTarget, GoTargetResolution, Locator, Outcome};
pub use interaction::{
    ActionContext, Interaction, Target, TargetFilter, TargetKind, TargetResolution, Verb,
};
pub use keys::dialogue_node_id::DialogueNodeId;
pub use keys::dialogue_option_id::DialogueOptionId;
pub use keys::npc_id::NpcId;
pub use keys::object_id::ObjectId;
pub use keys::room_id::RoomId;
pub use keys::trigger_id::TriggerId;
pub use rules::{BasicRules, Rules};
pub use world::npc::{DialogueGraph, DialogueNode, Npc};
pub use world::object::{ObjectInfo, ObjectResolution};
pub use world::{SaveError, WorldState};
