use getset::{Getters, MutGetters};
use std::collections::HashMap;

use crate::data;
use crate::input;
use crate::world::object;

pub use crate::model::room_id::RoomId;

pub const DIRECTIONS: [input::Direction; 4] = [
    input::Direction::North,
    input::Direction::South,
    input::Direction::East,
    input::Direction::West,
];

/// A single room: its visible and hidden objects, plus a door index derived
/// from them.
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
    /// Build a room, indexing its door objects (visible and hidden) by
    /// direction.
    pub(crate) fn new(
        objects: Vec<object::Object>,
        hidden_objects: Vec<object::Object>,
        extra: HashMap<String, data::ExtraValue>,
    ) -> Self {
        let mut directions = HashMap::new();
        for object in objects.iter().chain(hidden_objects.iter()) {
            if let Some(door) = &object.door {
                directions.insert(door.direction, object.id.clone());
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

/// Object lookup and mutation within the room.
impl Room {
    /// Find a *visible* object by id.
    pub(crate) fn get_object(&self, id: &object::ObjectId) -> object::ObjectResolution {
        match self.objects.iter().find(|object| object.id == *id) {
            Some(object) => object::ObjectResolution::Found(object.id.clone()),
            None => object::ObjectResolution::NotFound,
        }
    }

    /// Resolve `name` against the room's *visible* objects.
    pub(crate) fn find_object(&self, name: &str) -> object::ObjectResolution {
        object::Object::resolve_by_name(self.objects(), name)
    }

    /// Whether the *visible* object with `id` is in this room.
    pub(crate) fn holds(&self, id: &object::ObjectId) -> bool {
        self.objects.iter().any(|object| object.id == *id)
    }

    /// Find an object by id across visible and hidden contents.
    pub(crate) fn find_any(&self, id: &object::ObjectId) -> Option<&object::Object> {
        self.objects
            .iter()
            .chain(self.hidden_objects.iter())
            .find(|object| object.id == *id)
    }

    /// Mutably find an object by id across visible and hidden contents.
    pub(crate) fn find_any_mut(&mut self, id: &object::ObjectId) -> Option<&mut object::Object> {
        self.objects
            .iter_mut()
            .chain(self.hidden_objects.iter_mut())
            .find(|object| object.id == *id)
    }

    pub(crate) fn add_object(&mut self, object: object::Object) {
        self.objects_mut().push(object);
    }

    pub(crate) fn add_hidden_object(&mut self, object: object::Object) {
        self.hidden_objects_mut().push(object);
    }

    pub(crate) fn remove_object(&mut self, id: &object::ObjectId) -> Option<object::Object> {
        Room::remove_object_from_list(self.objects_mut(), id)
    }

    pub(crate) fn remove_hidden_object(&mut self, id: &object::ObjectId) -> Option<object::Object> {
        Room::remove_object_from_list(self.hidden_objects_mut(), id)
    }

    fn remove_object_from_list(
        objects: &mut Vec<object::Object>,
        id: &object::ObjectId,
    ) -> Option<object::Object> {
        let position = objects.iter().position(|object| object.id == *id);
        position.map(|pos| objects.remove(pos))
    }

    pub(crate) fn reveal_object(&mut self, id: &object::ObjectId) -> object::ObjectResolution {
        let removed = self.remove_hidden_object(id);
        match removed {
            Some(object) => {
                self.add_object(object);
                object::ObjectResolution::Found(id.clone())
            }
            None => object::ObjectResolution::NotFound,
        }
    }

    pub(crate) fn hide_object(&mut self, id: &object::ObjectId) -> object::ObjectResolution {
        let removed = self.remove_object(id);
        match removed {
            Some(object) => {
                self.add_hidden_object(object);
                object::ObjectResolution::Found(id.clone())
            }
            None => object::ObjectResolution::NotFound,
        }
    }
}

/// Door state and exit queries.
impl Room {
    /// The id of the scene object occupying `direction`, if any.
    fn door_id(&self, direction: input::Direction) -> Option<object::ObjectId> {
        self.directions.get(&direction).cloned()
    }

    /// The door object occupying `direction`, if any (visible or hidden).
    fn door_in_direction(&self, direction: input::Direction) -> Option<&object::Object> {
        self.door_id(direction).and_then(|id| self.find_any(&id))
    }

    fn door_in_direction_mut(
        &mut self,
        direction: input::Direction,
    ) -> Option<&mut object::Object> {
        let id = self.door_id(direction)?;
        self.find_any_mut(&id)
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
            Some(state.to.clone())
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

    /// Lock the door in `direction` (no-op if there is none, or it is
    /// already locked).
    pub(crate) fn lock_exit(&mut self, direction: input::Direction) -> input::DirectionResolution {
        match self.door_in_direction_mut(direction) {
            Some(object) if object.door.as_ref().is_some_and(|door| !door.locked) => {
                object.door.as_mut().expect("door ref").locked = true;
                input::DirectionResolution::Found(direction)
            }
            _ => input::DirectionResolution::NotFound,
        }
    }

    /// Unlock the door in `direction` (no-op if there is none, or it is
    /// already unlocked).
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
        let Some(id) = self.door_id(direction) else {
            return input::DirectionResolution::NotFound;
        };
        if self.is_exit_hidden(direction) {
            return input::DirectionResolution::NotFound;
        }
        match self.remove_object(&id) {
            Some(object) => {
                self.add_hidden_object(object);
                input::DirectionResolution::Found(direction)
            }
            None => input::DirectionResolution::NotFound,
        }
    }

    /// Reveal the hidden door in `direction` (no-op if there is none, or it
    /// is not hidden).
    pub(crate) fn reveal_exit(
        &mut self,
        direction: input::Direction,
    ) -> input::DirectionResolution {
        let Some(id) = self.door_id(direction) else {
            return input::DirectionResolution::NotFound;
        };
        if !self.is_exit_hidden(direction) {
            return input::DirectionResolution::NotFound;
        }
        match self.remove_hidden_object(&id) {
            Some(object) => {
                self.add_object(object);
                input::DirectionResolution::Found(direction)
            }
            None => input::DirectionResolution::NotFound,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::object_data::ObjectKind;
    use crate::world::object::{DoorState, Object};

    fn item(id: &str) -> object::Object {
        Object {
            id: object::ObjectId::new(id),
            primary_name: id.to_string(),
            aliases: vec![format!("{id}-alias")],
            kind: ObjectKind::Item,
            door: None,
            extra: HashMap::new(),
        }
    }

    fn door_object(
        id: &str,
        direction: input::Direction,
        to: &str,
        locked: bool,
    ) -> object::Object {
        Object {
            id: object::ObjectId::new(id),
            primary_name: id.to_string(),
            aliases: Vec::new(),
            kind: ObjectKind::Scene,
            door: Some(DoorState {
                direction,
                to: RoomId::new(to),
                locked,
            }),
            extra: HashMap::new(),
        }
    }

    #[test]
    fn new_indexes_doors_from_visible_and_hidden_objects() {
        let visible = door_object("north-door", input::Direction::North, "b", false);
        let hidden = door_object("south-door", input::Direction::South, "c", false);
        let room = Room::new(vec![visible], vec![hidden], HashMap::new());
        assert_eq!(
            room.door_id(input::Direction::North),
            Some(object::ObjectId::new("north-door"))
        );
        assert_eq!(
            room.door_id(input::Direction::South),
            Some(object::ObjectId::new("south-door"))
        );
        assert_eq!(room.door_id(input::Direction::East), None);
    }

    #[test]
    fn get_object_only_finds_visible_objects() {
        let room = Room::new(vec![item("sword")], vec![item("chest")], HashMap::new());
        assert_eq!(
            room.get_object(&object::ObjectId::new("sword")),
            object::ObjectResolution::Found(object::ObjectId::new("sword"))
        );
        assert_eq!(
            room.get_object(&object::ObjectId::new("chest")),
            object::ObjectResolution::NotFound
        );
    }

    #[test]
    fn find_object_resolves_by_name_or_alias_among_visible_objects() {
        let room = Room::new(vec![item("sword")], vec![item("chest")], HashMap::new());
        assert_eq!(
            room.find_object("sword"),
            object::ObjectResolution::Found(object::ObjectId::new("sword"))
        );
        assert_eq!(
            room.find_object("sword-alias"),
            object::ObjectResolution::Found(object::ObjectId::new("sword"))
        );
        assert_eq!(
            room.find_object("chest"),
            object::ObjectResolution::NotFound
        );
    }

    #[test]
    fn holds_checks_only_the_visible_list() {
        let room = Room::new(vec![item("sword")], vec![item("chest")], HashMap::new());
        assert!(room.holds(&object::ObjectId::new("sword")));
        assert!(!room.holds(&object::ObjectId::new("chest")));
    }

    #[test]
    fn find_any_searches_visible_and_hidden_objects() {
        let room = Room::new(vec![item("sword")], vec![item("chest")], HashMap::new());
        assert!(room.find_any(&object::ObjectId::new("sword")).is_some());
        assert!(room.find_any(&object::ObjectId::new("chest")).is_some());
        assert!(room.find_any(&object::ObjectId::new("nope")).is_none());
    }

    #[test]
    fn find_any_mut_allows_mutating_a_hidden_object() {
        let mut room = Room::new(vec![], vec![item("chest")], HashMap::new());
        let object = room
            .find_any_mut(&object::ObjectId::new("chest"))
            .expect("chest present");
        object.primary_name = "renamed chest".to_string();
        assert_eq!(
            room.find_any(&object::ObjectId::new("chest"))
                .unwrap()
                .primary_name,
            "renamed chest"
        );
    }

    #[test]
    fn add_object_and_add_hidden_object_insert_into_the_right_list() {
        let mut room = Room::default();
        room.add_object(item("sword"));
        room.add_hidden_object(item("chest"));
        assert!(room.holds(&object::ObjectId::new("sword")));
        assert!(!room.holds(&object::ObjectId::new("chest")));
        assert!(room.find_any(&object::ObjectId::new("chest")).is_some());
    }

    #[test]
    fn remove_object_takes_it_out_of_the_visible_list_only() {
        let mut room = Room::new(vec![item("sword")], vec![item("chest")], HashMap::new());
        assert!(
            room.remove_object(&object::ObjectId::new("chest"))
                .is_none()
        );
        let removed = room.remove_object(&object::ObjectId::new("sword"));
        assert_eq!(removed.map(|o| o.id), Some(object::ObjectId::new("sword")));
        assert!(!room.holds(&object::ObjectId::new("sword")));
    }

    #[test]
    fn remove_hidden_object_takes_it_out_of_the_hidden_list_only() {
        let mut room = Room::new(vec![item("sword")], vec![item("chest")], HashMap::new());
        assert!(
            room.remove_hidden_object(&object::ObjectId::new("sword"))
                .is_none()
        );
        let removed = room.remove_hidden_object(&object::ObjectId::new("chest"));
        assert_eq!(removed.map(|o| o.id), Some(object::ObjectId::new("chest")));
        assert!(room.find_any(&object::ObjectId::new("chest")).is_none());
    }

    #[test]
    fn reveal_object_moves_a_hidden_object_to_visible() {
        let mut room = Room::new(vec![], vec![item("chest")], HashMap::new());
        assert_eq!(
            room.reveal_object(&object::ObjectId::new("chest")),
            object::ObjectResolution::Found(object::ObjectId::new("chest"))
        );
        assert!(room.holds(&object::ObjectId::new("chest")));
    }

    #[test]
    fn reveal_object_not_hidden_returns_not_found() {
        let mut room = Room::default();
        assert_eq!(
            room.reveal_object(&object::ObjectId::new("nope")),
            object::ObjectResolution::NotFound
        );
    }

    #[test]
    fn hide_object_moves_a_visible_object_to_hidden() {
        let mut room = Room::new(vec![item("sword")], vec![], HashMap::new());
        assert_eq!(
            room.hide_object(&object::ObjectId::new("sword")),
            object::ObjectResolution::Found(object::ObjectId::new("sword"))
        );
        assert!(!room.holds(&object::ObjectId::new("sword")));
        assert!(room.find_any(&object::ObjectId::new("sword")).is_some());
    }

    #[test]
    fn hide_object_not_visible_returns_not_found() {
        let mut room = Room::default();
        assert_eq!(
            room.hide_object(&object::ObjectId::new("nope")),
            object::ObjectResolution::NotFound
        );
    }

    #[test]
    fn get_room_id_by_exit_direction_none_when_no_door() {
        let room = Room::default();
        assert_eq!(
            room.get_room_id_by_exit_direction(input::Direction::North),
            None
        );
    }

    #[test]
    fn get_room_id_by_exit_direction_none_when_locked_or_hidden() {
        let locked = door_object("locked-door", input::Direction::North, "b", true);
        let room = Room::new(vec![locked], vec![], HashMap::new());
        assert_eq!(
            room.get_room_id_by_exit_direction(input::Direction::North),
            None
        );

        let hidden = door_object("hidden-door", input::Direction::South, "c", false);
        let room = Room::new(vec![], vec![hidden], HashMap::new());
        assert_eq!(
            room.get_room_id_by_exit_direction(input::Direction::South),
            None
        );
    }

    #[test]
    fn get_room_id_by_exit_direction_returns_destination_for_open_door() {
        let open = door_object("open-door", input::Direction::East, "study", false);
        let room = Room::new(vec![open], vec![], HashMap::new());
        assert_eq!(
            room.get_room_id_by_exit_direction(input::Direction::East),
            Some(RoomId::new("study"))
        );
    }

    #[test]
    fn is_exit_locked_reports_the_door_lock_flag() {
        let locked = door_object("locked-door", input::Direction::North, "b", true);
        let unlocked = door_object("open-door", input::Direction::East, "b", false);
        let room = Room::new(vec![locked, unlocked], vec![], HashMap::new());
        assert!(room.is_exit_locked(input::Direction::North));
        assert!(!room.is_exit_locked(input::Direction::East));
        assert!(!room.is_exit_locked(input::Direction::South));
    }

    #[test]
    fn is_exit_hidden_reports_list_membership() {
        let visible = door_object("open-door", input::Direction::East, "b", false);
        let hidden = door_object("hidden-door", input::Direction::North, "b", false);
        let room = Room::new(vec![visible], vec![hidden], HashMap::new());
        assert!(room.is_exit_hidden(input::Direction::North));
        assert!(!room.is_exit_hidden(input::Direction::East));
    }

    #[test]
    fn exit_directions_lists_only_open_passable_doors() {
        let open = door_object("open-door", input::Direction::East, "b", false);
        let locked = door_object("locked-door", input::Direction::North, "b", true);
        let hidden = door_object("hidden-door", input::Direction::South, "b", false);
        let room = Room::new(vec![open, locked], vec![hidden], HashMap::new());
        assert_eq!(room.exit_directions(), vec![input::Direction::East]);
    }

    #[test]
    fn exit_extra_returns_the_door_objects_extra_data() {
        let mut extra = HashMap::new();
        extra.insert(
            "note".to_string(),
            data::ExtraValue::Str("creaky".to_string()),
        );
        let mut open = door_object("open-door", input::Direction::East, "b", false);
        open.extra = extra.clone();
        let room = Room::new(vec![open], vec![], HashMap::new());
        assert_eq!(room.exit_extra(input::Direction::East), Some(extra));
        assert_eq!(room.exit_extra(input::Direction::North), None);
    }

    #[test]
    fn door_in_direction_info_returns_none_for_a_missing_direction() {
        let room = Room::default();
        assert_eq!(room.door_in_direction_info(input::Direction::North), None);
    }

    #[test]
    fn lock_exit_locks_an_unlocked_door_and_is_a_no_op_otherwise() {
        let open = door_object("open-door", input::Direction::East, "b", false);
        let mut room = Room::new(vec![open], vec![], HashMap::new());
        assert_eq!(
            room.lock_exit(input::Direction::East),
            input::DirectionResolution::Found(input::Direction::East)
        );
        assert!(room.is_exit_locked(input::Direction::East));
        assert_eq!(
            room.lock_exit(input::Direction::East),
            input::DirectionResolution::NotFound
        );
    }

    #[test]
    fn lock_exit_missing_direction_is_not_found() {
        let mut room = Room::default();
        assert_eq!(
            room.lock_exit(input::Direction::East),
            input::DirectionResolution::NotFound
        );
    }

    #[test]
    fn unlock_exit_unlocks_a_locked_door_and_is_a_no_op_otherwise() {
        let locked = door_object("locked-door", input::Direction::East, "b", true);
        let mut room = Room::new(vec![locked], vec![], HashMap::new());
        assert_eq!(
            room.unlock_exit(input::Direction::East),
            input::DirectionResolution::Found(input::Direction::East)
        );
        assert!(!room.is_exit_locked(input::Direction::East));
        assert_eq!(
            room.unlock_exit(input::Direction::East),
            input::DirectionResolution::NotFound
        );
    }

    #[test]
    fn hide_exit_hides_a_visible_door_and_is_a_no_op_once_hidden() {
        let open = door_object("open-door", input::Direction::East, "b", false);
        let mut room = Room::new(vec![open], vec![], HashMap::new());
        assert_eq!(
            room.hide_exit(input::Direction::East),
            input::DirectionResolution::Found(input::Direction::East)
        );
        assert!(room.is_exit_hidden(input::Direction::East));
        assert_eq!(
            room.hide_exit(input::Direction::East),
            input::DirectionResolution::NotFound
        );
    }

    #[test]
    fn hide_exit_missing_direction_is_not_found() {
        let mut room = Room::default();
        assert_eq!(
            room.hide_exit(input::Direction::East),
            input::DirectionResolution::NotFound
        );
    }

    #[test]
    fn reveal_exit_reveals_a_hidden_door_and_is_a_no_op_once_visible() {
        let hidden = door_object("hidden-door", input::Direction::East, "b", false);
        let mut room = Room::new(vec![], vec![hidden], HashMap::new());
        assert_eq!(
            room.reveal_exit(input::Direction::East),
            input::DirectionResolution::Found(input::Direction::East)
        );
        assert!(!room.is_exit_hidden(input::Direction::East));
        assert_eq!(
            room.reveal_exit(input::Direction::East),
            input::DirectionResolution::NotFound
        );
    }

    #[test]
    fn reveal_exit_missing_direction_is_not_found() {
        let mut room = Room::default();
        assert_eq!(
            room.reveal_exit(input::Direction::East),
            input::DirectionResolution::NotFound
        );
    }
}
