pub mod object;
pub mod player;
pub mod room;

use std::collections::HashMap;

use getset::{CopyGetters, Getters};
use log::warn;

use crate::data;
use crate::data::ObjectKind;
use crate::input::action;
use crate::input::direction;
use crate::world::object::{ObjectId, ObjectInfo, ObjectResolution};

#[derive(Debug, Getters, CopyGetters)]
pub struct WorldState {
    #[getset(get = "pub")]
    player: player::Player,
    #[getset(get = "pub")]
    rooms: HashMap<room::RoomId, room::Room>,
    #[getset(get_copy = "pub")]
    current_room_id: room::RoomId,
}

/// Navigation: location and movement within the world.
impl WorldState {
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
    pub fn get_room_id_by_exit_direction(
        &self,
        direction: direction::Direction,
    ) -> Option<room::RoomId> {
        self.current_room().get_room_id_by_exit_direction(direction)
    }

    /// Change the current room to `room_id`, if it is known to the world.
    pub fn move_to_room(&mut self, room_id: room::RoomId) -> action::MoveResult {
        if self.rooms.contains_key(&room_id) {
            self.current_room_id = room_id;
            action::MoveResult::Success
        } else {
            warn!("Tried to move to unknown room (id: {})!", room_id);
            action::MoveResult::Fail
        }
    }

    /// Whether the exit in `direction` is locked (blocks traversal).
    pub fn is_exit_locked(&self, direction: direction::Direction) -> bool {
        self.current_room().is_exit_locked(direction)
    }

    /// Whether the exit in `direction` is hidden (not yet discovered by the player).
    pub fn is_exit_hidden(&self, direction: direction::Direction) -> bool {
        self.current_room().is_exit_hidden(direction)
    }

    /// The id of the object that unlocks the exit in `direction`, if one is
    /// declared. The gate link is data; unlocking behaviour is a rule.
    pub fn exit_gated_by(&self, direction: direction::Direction) -> Option<ObjectId> {
        self.current_room().exit_gated_by(direction)
    }

    /// Public details about the exit in `direction`, if there is one.
    pub fn exit_info(&self, direction: direction::Direction) -> Option<ObjectInfo> {
        self.current_room().door_in_direction_info(direction)
    }

    /// Directions with an open (passable) exit from the current room.
    ///
    /// Locked and hidden exits are excluded. Note: order is unspecified.
    pub fn exit_directions(&self) -> Vec<direction::Direction> {
        self.current_room().exit_directions()
    }

    /// The opaque `extra` data attached to the exit in `direction`, if any.
    pub fn exit_extra(
        &self,
        direction: direction::Direction,
    ) -> Option<HashMap<String, data::ExtraValue>> {
        self.current_room().exit_extra(direction)
    }

    pub fn unlock_exit(
        &mut self,
        direction: direction::Direction,
    ) -> direction::DirectionResolution {
        self.current_room_mut().unlock_exit(direction)
    }

    pub fn lock_exit(&mut self, direction: direction::Direction) -> direction::DirectionResolution {
        self.current_room_mut().lock_exit(direction)
    }

    pub fn reveal_exit(
        &mut self,
        direction: direction::Direction,
    ) -> direction::DirectionResolution {
        self.current_room_mut().reveal_exit(direction)
    }

    pub fn hide_exit(&mut self, direction: direction::Direction) -> direction::DirectionResolution {
        self.current_room_mut().hide_exit(direction)
    }
}

/// Room helpers
impl WorldState {
    pub fn get_object_from_room(&self, id: ObjectId) -> ObjectResolution {
        self.current_room().get_object(id)
    }

    pub fn remove_object_from_room(&mut self, id: ObjectId) -> Option<object::Object> {
        self.current_room_mut().remove_object(id)
    }

    /// Names of the objects currently visible in the current room.
    pub fn room_object_names(&self) -> Vec<String> {
        self.current_room()
            .objects()
            .iter()
            .map(|object| object.primary_name.clone())
            .collect()
    }

    pub fn reveal_object(&mut self, id: ObjectId) -> ObjectResolution {
        self.current_room_mut().reveal_object(id)
    }

    pub fn hide_object(&mut self, id: ObjectId) -> ObjectResolution {
        self.current_room_mut().hide_object(id)
    }

    pub fn current_room_extra(&self) -> HashMap<String, data::ExtraValue> {
        self.current_room().extra().clone()
    }
}

/// Player helpers
impl WorldState {
    pub fn get_object_from_player(&self, id: ObjectId) -> ObjectResolution {
        self.player.get_object(id)
    }

    pub fn remove_object_from_player(&mut self, id: ObjectId) -> Option<object::Object> {
        self.player.remove_object(id)
    }

    /// Whether the player currently holds the object with `id`.
    pub fn player_holds(&self, id: ObjectId) -> bool {
        self.player.holds(id)
    }

    /// Names of the objects currently held by the player.
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
    pub fn player_take_object(&mut self, id: ObjectId) -> action::TakeResult {
        let removed = self.remove_object_from_room(id);
        match removed {
            Some(object) => {
                self.player.add_object(object);
                action::TakeResult::Success
            }
            None => action::TakeResult::Fail,
        }
    }

    pub fn player_drop_object(&mut self, id: ObjectId) -> action::DropResult {
        let removed = self.remove_object_from_player(id);
        match removed {
            Some(object) => {
                self.current_room_mut().add_object(object);
                action::DropResult::Success
            }
            None => action::DropResult::Fail,
        }
    }
}

/// Object helpers over everything in the player's scope.
impl WorldState {
    /// Details about an object visible to the player (in the room or inventory).
    pub fn object_info(&self, id: ObjectId) -> Option<ObjectInfo> {
        self.any_object(id).map(ObjectInfo::from_object)
    }

    /// Find an object by id across the current room (visible or hidden) and
    /// the player's inventory.
    fn any_object(&self, id: ObjectId) -> Option<&object::Object> {
        self.current_room()
            .find_any(id)
            .or_else(|| self.player.find_by_id(id))
    }

    /// The kind of the object with `id`, if it is anywhere in scope.
    pub fn object_kind(&self, id: ObjectId) -> Option<ObjectKind> {
        self.any_object(id).map(|object| object.kind)
    }

    /// Whether the object with `id` is a scene object (stays in the world).
    pub fn object_is_scene(&self, id: ObjectId) -> bool {
        self.any_object(id)
            .is_some_and(|object| object.kind == ObjectKind::Scene)
    }

    /// Whether the object with `id` is a door (a scene object with door data).
    pub fn object_is_door(&self, id: ObjectId) -> bool {
        self.any_object(id)
            .is_some_and(|object| object.door.is_some())
    }

    /// The direction the door object with `id` occupies in its room, if it is
    /// a door. Works while the door is hidden too (the object is still in the
    /// world); hidden doors still do not resolve as targets.
    pub fn exit_direction_of(&self, id: ObjectId) -> Option<direction::Direction> {
        self.any_object(id)
            .and_then(|object| object.door.as_ref())
            .map(|door| door.direction)
    }

    /// Resolve a noun against everything currently in the player's scope:
    /// visible room objects and carried objects. Doors are ordinary scene
    /// objects, so they resolve here exactly like any other visible object.
    pub fn resolve_target(&self, name: &str) -> ObjectResolution {
        object::Object::resolve_by_name(&self.get_available_objects(), name)
    }

    /// Resolve a noun against the objects in the current room only.
    pub fn resolve_room_object(&self, name: &str) -> ObjectResolution {
        self.current_room().find_object(name)
    }

    /// Resolve a noun against the objects the player is carrying.
    pub fn resolve_player_object(&self, name: &str) -> ObjectResolution {
        self.player.find_object(name)
    }

    /// Whether a given target is currently in the player's scope.
    pub fn target_in_scope(&self, target: ObjectId) -> bool {
        self.current_room().holds(target) || self.player.holds(target)
    }

    fn get_available_objects(&self) -> Vec<object::Object> {
        [
            self.current_room().objects().as_slice(),
            self.player().objects().as_slice(),
        ]
        .concat()
    }
}

impl WorldState {
    /// Build a `WorldState` directly from world data (YAML), with no database.
    pub(crate) fn from_data(data: &data::WorldData) -> Self {
        let first_room_id: room::RoomId = data
            .rooms
            .first()
            .expect("world data must contain at least one room")
            .id
            .into();

        let mut rooms = HashMap::new();
        for room_data in &data.rooms {
            let objects: Vec<object::Object> = room_data
                .visible_objects
                .iter()
                .filter_map(|id| data.find_object(*id))
                .map(object::Object::from_data)
                .collect();

            let hidden_objects: Vec<object::Object> = room_data
                .hidden_objects
                .iter()
                .filter_map(|id| data.find_object(*id))
                .map(object::Object::from_data)
                .collect();

            rooms.insert(
                room_data.id.into(),
                room::Room::new(objects, hidden_objects, room_data.extra.clone()),
            );
        }

        if !rooms.contains_key(&first_room_id) {
            panic!("first room id {first_room_id} not found in rooms");
        }

        WorldState {
            player: player::Player::new(),
            rooms,
            current_room_id: first_room_id,
        }
    }
}
