use getset::{Getters, MutGetters};
use std::collections::HashMap;

use crate::data;
use crate::input;
use crate::world::object;

pub const DIRECTIONS: [input::Direction; 4] = [
    input::Direction::North,
    input::Direction::South,
    input::Direction::East,
    input::Direction::West,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RoomId(i32);

impl RoomId {
    pub fn new(value: i32) -> Self {
        RoomId(value)
    }

    pub fn get(self) -> i32 {
        self.0
    }
}

impl std::fmt::Display for RoomId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i32> for RoomId {
    fn from(value: i32) -> Self {
        RoomId::new(value)
    }
}

impl From<RoomId> for i32 {
    fn from(id: RoomId) -> i32 {
        id.get()
    }
}

#[derive(Debug, Getters, MutGetters, Default, Clone)]
#[getset(get = "pub(crate)")]
pub struct Room {
    #[get_mut(get_mut = "pub(crate)")]
    objects: Vec<object::Object>,
    #[get_mut(get_mut = "pub(crate)")]
    hidden_objects: Vec<object::Object>,
    /// Derived index: door direction → the scene object occupying it. Built
    /// from the room's door objects (visible + hidden) at construction; it is
    /// a cache for O(1) movement and door lookups, not a source of truth.
    directions: HashMap<input::Direction, object::ObjectId>,
    extra: HashMap<String, data::ExtraValue>,
}

impl Room {
    pub(crate) fn new(
        objects: Vec<object::Object>,
        hidden_objects: Vec<object::Object>,
        extra: HashMap<String, data::ExtraValue>,
    ) -> Self {
        let mut directions = HashMap::new();
        for object in objects.iter().chain(hidden_objects.iter()) {
            if let Some(door) = &object.door {
                directions.insert(door.direction, object.id);
            }
        }
        Room {
            objects,
            hidden_objects,
            directions,
            extra,
        }
    }
}

// Object management
impl Room {
    pub(crate) fn get_object(&self, id: object::ObjectId) -> object::ObjectResolution {
        match self.objects.iter().find(|object| object.id == id) {
            Some(object) => object::ObjectResolution::Found(object.id),
            None => object::ObjectResolution::NotFound,
        }
    }

    pub(crate) fn find_object(&self, name: &str) -> object::ObjectResolution {
        object::Object::resolve_by_name(self.objects(), name)
    }

    pub(crate) fn holds(&self, id: object::ObjectId) -> bool {
        self.objects.iter().any(|object| object.id == id)
    }

    /// Find an object by id across visible and hidden contents.
    pub(crate) fn find_any(&self, id: object::ObjectId) -> Option<&object::Object> {
        self.objects
            .iter()
            .chain(self.hidden_objects.iter())
            .find(|object| object.id == id)
    }

    /// Mutably find an object by id across visible and hidden contents.
    pub(crate) fn find_any_mut(&mut self, id: object::ObjectId) -> Option<&mut object::Object> {
        self.objects
            .iter_mut()
            .chain(self.hidden_objects.iter_mut())
            .find(|object| object.id == id)
    }

    pub(crate) fn add_object(&mut self, object: object::Object) {
        self.objects_mut().push(object);
    }

    pub(crate) fn add_hidden_object(&mut self, object: object::Object) {
        self.hidden_objects_mut().push(object);
    }

    pub(crate) fn remove_object(&mut self, id: object::ObjectId) -> Option<object::Object> {
        Room::remove_object_from_list(self.objects_mut(), id)
    }

    pub(crate) fn remove_hidden_object(&mut self, id: object::ObjectId) -> Option<object::Object> {
        Room::remove_object_from_list(self.hidden_objects_mut(), id)
    }

    fn remove_object_from_list(
        objects: &mut Vec<object::Object>,
        id: object::ObjectId,
    ) -> Option<object::Object> {
        let position = objects.iter().position(|object| object.id == id);
        position.map(|pos| objects.remove(pos))
    }

    pub(crate) fn reveal_object(&mut self, id: object::ObjectId) -> object::ObjectResolution {
        let removed = self.remove_hidden_object(id);
        match removed {
            Some(object) => {
                self.add_object(object);
                object::ObjectResolution::Found(id)
            }
            None => object::ObjectResolution::NotFound,
        }
    }

    pub(crate) fn hide_object(&mut self, id: object::ObjectId) -> object::ObjectResolution {
        let removed = self.remove_object(id);
        match removed {
            Some(object) => {
                self.add_hidden_object(object);
                object::ObjectResolution::Found(id)
            }
            None => object::ObjectResolution::NotFound,
        }
    }
}

// Door management
impl Room {
    /// The id of the scene object occupying `direction`, if any.
    fn door_id(&self, direction: input::Direction) -> Option<object::ObjectId> {
        self.directions.get(&direction).copied()
    }

    /// The door object occupying `direction`, if any (visible or hidden).
    fn door_in_direction(&self, direction: input::Direction) -> Option<&object::Object> {
        self.door_id(direction).and_then(|id| self.find_any(id))
    }

    fn door_in_direction_mut(
        &mut self,
        direction: input::Direction,
    ) -> Option<&mut object::Object> {
        let id = self.door_id(direction)?;
        self.find_any_mut(id)
    }

    /// The destination of an *open* exit in `direction`, if one exists.
    ///
    /// Hidden and locked exits are not usable for movement, so they resolve to
    /// `None` (exactly as if no exit were present).
    pub(crate) fn get_room_id_by_exit_direction(
        &self,
        direction: input::Direction,
    ) -> Option<RoomId> {
        let door = self.door_in_direction(direction)?;
        let state = door.door.as_ref()?;
        if state.locked || self.is_exit_hidden(direction) {
            None
        } else {
            Some(state.to)
        }
    }

    pub(crate) fn is_exit_locked(&self, direction: input::Direction) -> bool {
        self.door_in_direction(direction)
            .is_some_and(|object| object.door.as_ref().is_some_and(|door| door.locked))
    }

    /// Whether the door in `direction` is hidden: it exists, but lives in the
    /// room's `hidden_objects` until revealed.
    pub(crate) fn is_exit_hidden(&self, direction: input::Direction) -> bool {
        self.door_id(direction)
            .is_some_and(|id| self.hidden_objects.iter().any(|object| object.id == id))
    }

    /// The id of the object that unlocks the door in `direction`, if one is
    /// declared (i.e. the gate exists).
    pub(crate) fn exit_gated_by(&self, direction: input::Direction) -> Option<object::ObjectId> {
        self.door_in_direction(direction)
            .and_then(|object| object.door.as_ref())
            .and_then(|door| door.gated_by)
    }

    /// Directions leading to an *open* (passable) exit in this room.
    pub(crate) fn exit_directions(&self) -> Vec<input::Direction> {
        DIRECTIONS
            .iter()
            .copied()
            .filter(|direction| {
                self.door_in_direction(*direction).is_some()
                    && !self.is_exit_locked(*direction)
                    && !self.is_exit_hidden(*direction)
            })
            .collect()
    }

    pub(crate) fn exit_extra(
        &self,
        direction: input::Direction,
    ) -> Option<HashMap<String, data::ExtraValue>> {
        self.door_in_direction(direction)
            .map(|object| object.extra.clone())
    }

    /// Public view of the door object in `direction`, if any (visible or hidden).
    pub(crate) fn door_in_direction_info(
        &self,
        direction: input::Direction,
    ) -> Option<object::ObjectInfo> {
        self.door_in_direction(direction)
            .map(object::ObjectInfo::from_object)
    }

    pub(crate) fn lock_exit(&mut self, direction: input::Direction) -> input::DirectionResolution {
        match self.door_in_direction_mut(direction) {
            Some(object) if object.door.as_ref().is_some_and(|door| !door.locked) => {
                object.door.as_mut().expect("door ref").locked = true;
                input::DirectionResolution::Found(direction)
            }
            _ => input::DirectionResolution::NotFound,
        }
    }

    pub(crate) fn unlock_exit(
        &mut self,
        direction: input::Direction,
    ) -> input::DirectionResolution {
        match self.door_in_direction_mut(direction) {
            Some(object) if object.door.as_ref().is_some_and(|door| door.locked) => {
                object.door.as_mut().expect("door ref").locked = false;
                input::DirectionResolution::Found(direction)
            }
            _ => input::DirectionResolution::NotFound,
        }
    }

    /// Hide the door in `direction` by moving its object out of the visible
    /// contents. Hidden-ness is list membership; the door object itself keeps
    /// its state.
    pub(crate) fn hide_exit(&mut self, direction: input::Direction) -> input::DirectionResolution {
        let id = match self.door_id(direction) {
            Some(id) => id,
            None => return input::DirectionResolution::NotFound,
        };
        if self.is_exit_hidden(direction) {
            return input::DirectionResolution::NotFound;
        }
        match self.remove_object(id) {
            Some(object) => {
                self.add_hidden_object(object);
                input::DirectionResolution::Found(direction)
            }
            None => input::DirectionResolution::NotFound,
        }
    }

    pub(crate) fn reveal_exit(
        &mut self,
        direction: input::Direction,
    ) -> input::DirectionResolution {
        let id = match self.door_id(direction) {
            Some(id) => id,
            None => return input::DirectionResolution::NotFound,
        };
        if !self.is_exit_hidden(direction) {
            return input::DirectionResolution::NotFound;
        }
        match self.remove_hidden_object(id) {
            Some(object) => {
                self.add_object(object);
                input::DirectionResolution::Found(direction)
            }
            None => input::DirectionResolution::NotFound,
        }
    }
}
