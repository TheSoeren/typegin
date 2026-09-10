use crate::world::npc::NpcId;
use crate::world::object::ObjectId;

use super::{Target, Verb};

/// Runtime context handed to an interaction: which object was used/dropped and
/// which target (if any) the verb was directed at.
///
/// For `use X on Y`: `item` is the carried `X`, `target` is the resolved `Y`
/// (object or NPC). For a self-use (`use X`), `target` is `None`.
#[derive(Debug, Clone)]
pub struct ActionContext {
    pub verb: Option<Verb>,
    pub item: Option<ObjectId>,
    pub target: Option<Target>,
}

impl ActionContext {
    #[must_use]
    pub fn new(verb: Option<Verb>, item: Option<ObjectId>, target: Option<Target>) -> Self {
        ActionContext { verb, item, target }
    }

    /// The object this context's target denotes, when the target is an object.
    #[must_use]
    pub fn target_object(&self) -> Option<&ObjectId> {
        match &self.target {
            Some(Target::Object(id)) => Some(id),
            _ => None,
        }
    }

    /// The NPC this context's target denotes, when the target is an NPC.
    #[must_use]
    pub fn target_npc(&self) -> Option<&NpcId> {
        match &self.target {
            Some(Target::Npc(id)) => Some(id),
            _ => None,
        }
    }
}
