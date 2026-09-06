use std::collections::HashMap;

use crate::data;
use crate::data::ObjectKind;
use crate::input::direction::Direction;
use crate::world::room::RoomId;

/// Identifier for a world object. All interactables share one id space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ObjectId(pub(crate) i32);

impl ObjectId {
    pub fn new(value: i32) -> Self {
        ObjectId(value)
    }

    pub fn get(self) -> i32 {
        self.0
    }
}

impl From<i32> for ObjectId {
    fn from(value: i32) -> Self {
        ObjectId::new(value)
    }
}

impl From<ObjectId> for i32 {
    fn from(id: ObjectId) -> Self {
        id.get()
    }
}

/// Outcome of resolving a player-typed noun against the world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectResolution {
    Found(ObjectId),
    Ambiguous { ids: Vec<ObjectId>, alias: String },
    NotFound,
}

/// A world object: gameplay-flavoured "thing" that can be carried, examined
/// and used. Contains only *facts* (identity, names, kind, opaque extras) — all
/// behaviour (what happens when you use X on Y) lives in rules and
/// interactions, not here.
#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    pub(crate) id: ObjectId,
    pub(crate) primary_name: String,
    pub(crate) aliases: String,
    pub(crate) kind: ObjectKind,
    pub(crate) door: Option<DoorState>,
    pub(crate) extra: HashMap<String, data::ExtraValue>,
}

/// Runtime door state of a scene object: the direction it occupies, its
/// destination, and whether it is locked. Hidden-ness is *not* stored here —
/// a hidden door is simply an object living in the room's `hidden_objects`.
#[derive(Debug, Clone, PartialEq)]
pub struct DoorState {
    pub(crate) direction: Direction,
    pub(crate) to: RoomId,
    pub(crate) locked: bool,
    pub(crate) gated_by: Option<ObjectId>,
}

impl Object {
    pub fn has_name(&self, name: &str) -> bool {
        self.primary_name == name || self.aliases.split(';').any(|alias| alias == name)
    }

    pub fn resolve_by_name(objects: &[Object], name: &str) -> ObjectResolution {
        let matching: Vec<ObjectId> = objects
            .iter()
            .filter(|object| object.has_name(name))
            .map(|object| object.id)
            .collect();

        match matching.len() {
            0 => ObjectResolution::NotFound,
            1 => ObjectResolution::Found(matching[0]),
            _ => ObjectResolution::Ambiguous {
                ids: matching,
                alias: name.to_string(),
            },
        }
    }

    pub(crate) fn from_data(object: &data::ObjectData) -> Self {
        // A door is inherently a scene object: it stays in the world and is
        // never portable. Declaring door data forces the kind, so the two-kind
        // invariant always holds at runtime.
        let kind = if object.door.is_some() {
            ObjectKind::Scene
        } else {
            object.kind
        };
        let door = object.door.as_ref().and_then(|door_data| {
            Direction::parse(&door_data.direction).map(|direction| DoorState {
                direction,
                to: door_data.to.into(),
                locked: door_data.locked,
                gated_by: door_data.gated_by.map(ObjectId::from),
            })
        });
        Object {
            id: object.id.into(),
            primary_name: object.primary_name.clone(),
            aliases: object.aliases.join(";"),
            kind,
            door,
            extra: object.extra.clone(),
        }
    }
}

/// Public, plain-data view of an object in the world, handed to game rules so
/// they can decide behaviour without reaching into the engine's internals.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectInfo {
    pub id: ObjectId,
    pub name: String,
    pub aliases: Vec<String>,
    pub kind: ObjectKind,
    pub door: Option<DoorInfo>,
    pub extra: HashMap<String, data::ExtraValue>,
}

/// Public, plain-data view of a door on a scene object.
#[derive(Debug, Clone, PartialEq)]
pub struct DoorInfo {
    pub direction: Direction,
    pub to: RoomId,
    pub locked: bool,
    pub gated_by: Option<ObjectId>,
}

impl ObjectInfo {
    pub(crate) fn from_object(object: &Object) -> Self {
        ObjectInfo {
            id: object.id,
            name: object.primary_name.clone(),
            aliases: object
                .aliases
                .split(';')
                .filter(|a| !a.is_empty())
                .map(str::to_string)
                .collect(),
            kind: object.kind,
            door: object.door.as_ref().map(|door| DoorInfo {
                direction: door.direction,
                to: door.to,
                locked: door.locked,
                gated_by: door.gated_by,
            }),
            extra: object.extra.clone(),
        }
    }
}
