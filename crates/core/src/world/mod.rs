pub mod npc;
pub mod object;
pub mod player;
pub mod room;

use std::collections::{HashMap, HashSet};

use getset::Getters;
use log::warn;

use crate::Target;
use crate::data;
use crate::data::interactions_data::InteractionData;
use crate::data::object_data;
use crate::data::trigger_data::TriggerData;
use crate::input::action;
use crate::input::direction;
use crate::keys::dialogue_node_id::DialogueNodeId;
use crate::keys::object_id::ObjectId;
use crate::keys::trigger_id::TriggerId;
use crate::world::object::TargetResolution;
use crate::world::object::{ObjectInfo, ObjectResolution};

/// The mutable authority for a running game: rooms, the player's inventory,
/// global flags, and NPC dialogue state. All gameplay mutation goes through
/// its methods; [`Rules`](crate::rules::Rules) and
/// [`Interaction`](crate::interaction::Interaction) effects are the only
/// callers that hold a `&mut WorldState`.
#[derive(Debug, Getters)]
pub struct WorldState {
    flags: Vec<String>,
    player: player::Player,
    rooms: HashMap<room::RoomId, room::Room>,
    current_room_id: room::RoomId,
    /// Authored data-driven interactions shipped in world data. Queried by the
    /// default rules hooks *before* `Rules::interactions()` closures.
    data_interactions: Vec<InteractionData>,
    /// Authored triggers shipped in world data, in declaration order. Checked
    /// after every action by `crate::trigger::check_triggers`.
    triggers: Vec<TriggerData>,
    /// Ids of triggers that have already fired. Each trigger fires at most
    /// once, ever (no re-arm).
    fired_triggers: HashSet<TriggerId>,
    /// Object templates from world data, used to materialise an object into
    /// the player's inventory that is not placed in any room (`grant`).
    object_templates: HashMap<ObjectId, object::Object>,
    /// NPCs loaded from world data.
    #[getset(get = "pub")]
    npcs: Vec<npc::Npc>,
    /// Current dialogue node per NPC. The most recently talked-to NPC is the
    /// "active" one for `Choose` dispatch.
    dialogue_state: HashMap<npc::NpcId, DialogueNodeId>,
    /// The NPC the player is currently talking to; `Choose` dispatch reads
    /// this NPC's current node from `dialogue_state`.
    #[getset(get = "pub")]
    active_npc: Option<npc::NpcId>,
}

/// Navigation: location and movement within the world.
impl WorldState {
    /// The id of the room the player is currently in.
    #[must_use]
    pub fn current_room_id(&self) -> room::RoomId {
        self.current_room_id.clone()
    }
    /// The room the player is currently in.
    fn current_room(&self) -> &room::Room {
        self.rooms
            .get(&self.current_room_id)
            .expect("current room must be present in the world")
    }

    /// The mutable room the player is currently in.
    fn current_room_mut(&mut self) -> &mut room::Room {
        self.rooms
            .get_mut(&self.current_room_id)
            .expect("current room must be present in the world")
    }

    /// Resolve which room an open exit direction leads to from the current room.
    ///
    /// Hidden and locked exits are not traversable, so they resolve to `None`
    /// just like a direction with no exit at all.
    #[must_use]
    pub fn get_room_id_by_exit_direction(
        &self,
        direction: direction::Direction,
    ) -> Option<room::RoomId> {
        self.current_room().get_room_id_by_exit_direction(direction)
    }

    /// Change the current room to `room_id`, if it is known to the world.
    ///
    /// Moving rooms ends any active conversation.
    pub fn move_to_room(&mut self, room_id: room::RoomId) -> action::Outcome {
        if self.rooms.contains_key(&room_id) {
            self.current_room_id = room_id;
            self.clear_all_dialogue();
            action::Outcome::Success
        } else {
            warn!("Tried to move to unknown room (id: {room_id})!");
            action::Outcome::Fail
        }
    }

    /// Whether the exit in `direction` is locked (blocks traversal).
    #[must_use]
    pub fn is_exit_locked(&self, direction: direction::Direction) -> bool {
        self.current_room().is_exit_locked(direction)
    }

    /// Whether the exit in `direction` is hidden (not yet discovered by the player).
    #[must_use]
    pub fn is_exit_hidden(&self, direction: direction::Direction) -> bool {
        self.current_room().is_exit_hidden(direction)
    }

    /// Public details about the exit in `direction`, if there is one.
    #[must_use]
    pub fn exit_info(&self, direction: direction::Direction) -> Option<ObjectInfo> {
        self.current_room().door_in_direction_info(direction)
    }

    /// Directions with an open (passable) exit from the current room.
    ///
    /// Locked and hidden exits are excluded. Note: order is unspecified.
    #[must_use]
    pub fn exit_directions(&self) -> Vec<direction::Direction> {
        self.current_room().exit_directions()
    }

    /// The opaque `extra` data attached to the exit in `direction`, if any.
    #[must_use]
    pub fn exit_extra(
        &self,
        direction: direction::Direction,
    ) -> Option<HashMap<String, data::ExtraValue>> {
        self.current_room().exit_extra(direction)
    }

    /// Unlock the exit in `direction` (no-op if there is none, or it is
    /// already unlocked).
    pub fn unlock_exit(
        &mut self,
        direction: direction::Direction,
    ) -> direction::DirectionResolution {
        self.current_room_mut().unlock_exit(direction)
    }

    /// Lock the exit in `direction` (no-op if there is none, or it is already
    /// locked).
    pub fn lock_exit(&mut self, direction: direction::Direction) -> direction::DirectionResolution {
        self.current_room_mut().lock_exit(direction)
    }

    /// Reveal the hidden exit in `direction` (no-op if there is none, or it
    /// is not hidden).
    pub fn reveal_exit(
        &mut self,
        direction: direction::Direction,
    ) -> direction::DirectionResolution {
        self.current_room_mut().reveal_exit(direction)
    }

    /// Hide the exit in `direction` (no-op if there is none, or it is already
    /// hidden).
    pub fn hide_exit(&mut self, direction: direction::Direction) -> direction::DirectionResolution {
        self.current_room_mut().hide_exit(direction)
    }
}

/// Room helpers
impl WorldState {
    #[must_use]
    pub fn get_object_from_room(&self, id: &ObjectId) -> ObjectResolution {
        self.current_room().get_object(id)
    }

    pub fn remove_object_from_room(&mut self, id: &ObjectId) -> Option<object::Object> {
        self.current_room_mut().remove_object(id)
    }

    /// Names of the objects currently visible in the current room.
    #[must_use]
    pub fn room_object_names(&self) -> Vec<String> {
        self.current_room()
            .objects()
            .iter()
            .map(|object| object.primary_name.clone())
            .collect()
    }

    pub fn reveal_object(&mut self, id: &ObjectId) -> ObjectResolution {
        self.current_room_mut().reveal_object(id)
    }

    pub fn hide_object(&mut self, id: &ObjectId) -> ObjectResolution {
        self.current_room_mut().hide_object(id)
    }

    #[must_use]
    pub fn current_room_extra(&self) -> HashMap<String, data::ExtraValue> {
        self.current_room().extra().clone()
    }
}

/// Player helpers
impl WorldState {
    #[must_use]
    pub fn get_object_from_player(&self, id: &ObjectId) -> ObjectResolution {
        self.player.get_object(id)
    }

    pub fn remove_object_from_player(&mut self, id: &ObjectId) -> Option<object::Object> {
        self.player.remove_object(id)
    }

    /// Whether the player currently holds the object with `id`.
    #[must_use]
    pub fn player_holds(&self, id: &ObjectId) -> bool {
        self.player.holds(id)
    }

    /// Names of the objects currently held by the player.
    #[must_use]
    pub fn player_object_names(&self) -> Vec<String> {
        self.player
            .objects()
            .iter()
            .map(|object| object.primary_name.clone())
            .collect()
    }
}

/// Object transfer management
impl WorldState {
    pub fn player_take_object(&mut self, id: &ObjectId) -> action::Outcome {
        let removed = self.remove_object_from_room(id);
        match removed {
            Some(object) => {
                self.player.add_object(object);
                action::Outcome::Success
            }
            None => action::Outcome::Fail,
        }
    }

    /// Materialise the object with `id` into the player's inventory from the
    /// world-data object templates, regardless of where (if anywhere) the
    /// object is placed. A no-op if the player already holds the object.
    pub fn player_grant_object(&mut self, id: &ObjectId) -> action::Outcome {
        if self.player_holds(id) {
            return action::Outcome::Fail;
        }
        let Some(template) = self.object_templates.get(id) else {
            return action::Outcome::Fail;
        };
        self.player.add_object(template.clone());
        action::Outcome::Success
    }

    pub fn player_drop_object(&mut self, id: &ObjectId) -> action::Outcome {
        let removed = self.remove_object_from_player(id);
        match removed {
            Some(object) => {
                self.current_room_mut().add_object(object);
                action::Outcome::Success
            }
            None => action::Outcome::Fail,
        }
    }

    /// Remove the carried object with `id` from the player's inventory without
    /// placing it in the room (consumed). A no-op if the object is not carried.
    pub fn player_discard_object(&mut self, id: &ObjectId) -> action::Outcome {
        match self.remove_object_from_player(id) {
            Some(_) => action::Outcome::Success,
            None => action::Outcome::Fail,
        }
    }
}

/// Object helpers over everything in the player's scope.
impl WorldState {
    /// Details about an object visible to the player (in the room or inventory).
    pub fn object_info(&self, id: &ObjectId) -> Option<ObjectInfo> {
        self.any_object(id).map(ObjectInfo::from_object)
    }

    /// Find an object by id across the current room (visible or hidden) and
    /// the player's inventory.
    fn any_object(&self, id: &ObjectId) -> Option<&object::Object> {
        self.current_room()
            .find_any(id)
            .or_else(|| self.player.find_by_id(id))
    }

    /// The kind of the object with `id`, if it is anywhere in scope.
    #[must_use]
    pub fn object_kind(&self, id: &ObjectId) -> Option<object_data::ObjectKind> {
        self.any_object(id).map(|object| object.kind)
    }

    /// Whether the object with `id` is a scene object (stays in the world).
    #[must_use]
    pub fn object_is_scene(&self, id: &ObjectId) -> bool {
        self.any_object(id)
            .is_some_and(|object| object.kind == object_data::ObjectKind::Scene)
    }

    /// Whether the object with `id` is a door (a scene object with door data).
    #[must_use]
    pub fn object_is_door(&self, id: &ObjectId) -> bool {
        self.any_object(id)
            .is_some_and(|object| object.door.is_some())
    }

    /// The direction the door object with `id` occupies in its room, if it is
    /// a door. Works while the door is hidden too (the object is still in the
    /// world); hidden doors still do not resolve as targets.
    #[must_use]
    pub fn exit_direction_of(&self, id: &ObjectId) -> Option<direction::Direction> {
        self.any_object(id)
            .and_then(|object| object.door.as_ref())
            .map(|door| door.direction)
    }

    /// Resolve a noun against everything currently in the player's scope:
    /// visible room objects and carried objects. Doors are ordinary scene
    /// objects, so they resolve here exactly like any other visible object.
    #[must_use]
    pub fn resolve_target(&self, name: &str) -> TargetResolution {
        match self.resolve_npc(name) {
            Some(npc) => TargetResolution::Found(Target::Npc(npc.id.clone())),
            None => match object::Object::resolve_by_name(&self.get_available_objects(), name) {
                ObjectResolution::Found(object_id) => {
                    TargetResolution::Found(Target::Object(object_id))
                }
                ObjectResolution::Ambiguous { ids, alias } => TargetResolution::Ambiguous {
                    ids: ids.iter().map(|id| Target::Object(id.clone())).collect(),
                    alias,
                },
                ObjectResolution::NotFound => TargetResolution::NotFound,
            },
        }
    }

    /// Resolve a noun against the objects in the current room only.
    #[must_use]
    pub fn resolve_room_object(&self, name: &str) -> ObjectResolution {
        self.current_room().find_object(name)
    }

    /// Resolve a noun against the objects the player is carrying.
    #[must_use]
    pub fn resolve_player_object(&self, name: &str) -> ObjectResolution {
        self.player.find_object(name)
    }

    /// Whether a given target is currently in the player's scope.
    #[must_use]
    pub fn target_in_scope(&self, target: &ObjectId) -> bool {
        self.current_room().holds(target) || self.player.holds(target)
    }

    fn get_available_objects(&self) -> Vec<object::Object> {
        [
            self.current_room().objects().as_slice(),
            self.player.objects().as_slice(),
        ]
        .concat()
    }
}

/// Global flags
impl WorldState {
    #[must_use]
    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags.iter().any(|f| f == flag)
    }

    pub fn set_flag(&mut self, flag: &str) {
        if !self.has_flag(flag) {
            self.flags.push(flag.to_string());
        }
    }

    pub fn clear_flag(&mut self, flag: &str) {
        self.flags.retain(|f| f != flag);
    }
}

/// NPC helpers
impl WorldState {
    /// NPCs present in the given room.
    #[must_use]
    pub fn npcs_in_room(&self, room_id: &room::RoomId) -> Vec<&npc::Npc> {
        self.npcs
            .iter()
            .filter(|npc| npc.room_id() == room_id)
            .collect()
    }

    /// Resolve an NPC by name (primary name or alias) in the current room.
    #[must_use]
    pub fn resolve_npc(&self, name: &str) -> Option<&npc::Npc> {
        self.npcs_in_room(&self.current_room_id)
            .into_iter()
            .find(|npc| npc.has_name(name))
    }

    /// The current dialogue node id for the given NPC, if in conversation.
    #[must_use]
    pub fn npc_dialogue_node(&self, npc_id: &npc::NpcId) -> Option<&DialogueNodeId> {
        self.dialogue_state.get(npc_id)
    }

    /// Set the dialogue node for an NPC.
    pub fn set_dialogue_node(&mut self, npc_id: npc::NpcId, node_id: DialogueNodeId) {
        self.dialogue_state.insert(npc_id, node_id);
    }

    /// Clear the dialogue state for an NPC (end conversation).
    pub fn clear_dialogue(&mut self, npc_id: &npc::NpcId) {
        self.dialogue_state.remove(npc_id);
    }

    /// Clear all dialogue state (e.g. on room change).
    pub fn clear_all_dialogue(&mut self) {
        self.dialogue_state.clear();
        self.active_npc = None;
    }

    /// Mark the NPC the player is currently talking to.
    pub fn set_active_npc(&mut self, npc_id: npc::NpcId) {
        self.active_npc = Some(npc_id);
    }

    /// Clear the active NPC (e.g. when a conversation ends).
    pub fn clear_active_npc(&mut self) {
        self.active_npc = None;
    }
}

/// Data-driven interaction access.
impl WorldState {
    /// The authored data-driven interactions, in declaration order.
    pub(crate) fn data_interactions(&self) -> &[InteractionData] {
        &self.data_interactions
    }
}

/// Trigger access.
impl WorldState {
    /// The authored triggers, in declaration order.
    pub(crate) fn triggers(&self) -> &[TriggerData] {
        &self.triggers
    }

    /// Whether the trigger with `id` has already fired.
    #[must_use]
    pub(crate) fn trigger_fired(&self, id: &TriggerId) -> bool {
        self.fired_triggers.contains(id)
    }

    /// Mark the trigger with `id` as fired (idempotent).
    pub(crate) fn mark_trigger_fired(&mut self, id: TriggerId) {
        self.fired_triggers.insert(id);
    }
}

/// Data import
impl WorldState {
    /// Build a `WorldState` directly from world data (YAML), with no database.
    pub(crate) fn from_data(data: &data::WorldData) -> Self {
        let first_room_id = data
            .rooms
            .first()
            .expect("world data must contain at least one room")
            .id
            .clone();

        let object_templates: HashMap<ObjectId, object::Object> = data
            .objects
            .iter()
            .map(object::Object::from_data)
            .map(|object| (object.id.clone(), object))
            .collect();

        let mut rooms = HashMap::new();
        for room_data in &data.rooms {
            let objects: Vec<object::Object> = room_data
                .visible_objects
                .iter()
                .filter_map(|id| data.find_object(id))
                .map(object::Object::from_data)
                .collect();

            let hidden_objects: Vec<object::Object> = room_data
                .hidden_objects
                .iter()
                .filter_map(|id| data.find_object(id))
                .map(object::Object::from_data)
                .collect();

            rooms.insert(
                room_data.id.clone(),
                room::Room::new(objects, hidden_objects, room_data.extra.clone()),
            );
        }

        assert!(
            rooms.contains_key(&first_room_id),
            "first room id {first_room_id} not found in rooms"
        );

        WorldState {
            flags: data.flags.clone(),
            player: player::Player::new(),
            rooms,
            current_room_id: first_room_id,
            data_interactions: data.interactions.clone(),
            triggers: data.triggers.clone(),
            fired_triggers: HashSet::new(),
            object_templates,
            npcs: data.npcs.iter().map(npc::Npc::from_data).collect(),
            dialogue_state: HashMap::new(),
            active_npc: None,
        }
    }
}
