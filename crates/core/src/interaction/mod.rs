mod action_context;
mod data_interactions;
#[allow(clippy::module_inception)]
mod interaction;
mod target;
mod verb;

pub(crate) use data_interactions::dispatch_data;

pub use action_context::ActionContext;
pub use interaction::{Interaction, InteractionCondition, InteractionEffect};
pub use target::{Target, TargetFilter, TargetKind, TargetResolution};
pub use verb::Verb;
