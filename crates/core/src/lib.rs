//! Front-end-agnostic text-adventure engine: parsing, world state, and a
//! data-driven interaction system, behind a [`View`] trait so a game can be
//! played as text, GUI, or point-and-click without changing engine code.

pub mod data;
pub mod engine;
pub mod event;
pub mod input;
pub mod interaction;
pub mod model;
pub mod rules;
pub mod view;
pub mod world;

pub use data::interactions_data::{
    DataCondition, DataEffect, DataTarget, DataTargetKind, InteractionData,
};
pub use data::npc_data::{DialogueChoiceData, DialogueData, DialogueNodeData, NpcData};
pub use data::object_data;
pub use data::{ExtraValue, WorldData, WorldDataError};
pub use engine::GameEngine;
pub use event::{DialogueChoice, Event};
pub use input::parse_input;
pub use input::{Action, Direction, DirectionResolution, DropResult, MoveResult, TakeResult};
pub use interaction::{ActionContext, Interaction, Target, TargetFilter, TargetResolution, Verb};
pub use model::dialogue_node_id::DialogueNodeId;
pub use model::dialogue_option_id::DialogueOptionId;
pub use model::npc_id::NpcId;
pub use model::object_id::ObjectId;
pub use model::room_id::RoomId;
pub use rules::{BasicRules, Rules};
pub use view::{RenderCommand, View};
pub use world::WorldState;
pub use world::npc::{DialogueGraph, DialogueNode, Npc};
pub use world::object::{ObjectInfo, ObjectResolution};
