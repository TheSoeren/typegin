use std::collections::HashMap;

use crate::data::{self, object_data};
use crate::input::direction::Direction;
use crate::world::room::RoomId;

pub use crate::keys::object_id::ObjectId;

/// Outcome of resolving a player-typed noun against the objects in scope.
pub type ObjectResolution = crate::keys::Resolution<ObjectId>;

/// Outcome of resolving a player-typed noun against a use-with target
/// (object or NPC). Defined next to [`Target`](crate::interaction::Target)
/// in `interaction`; re-exported here for the established `object::` path.
pub use crate::interaction::TargetResolution;

/// A world object: gameplay-flavoured "thing" that can be carried, examined
/// and used. Contains only *facts* (identity, names, kind, opaque extras) — all
/// behaviour (what happens when you use X on Y) lives in rules and
/// interactions, not here.
#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    pub(crate) id: ObjectId,
    pub(crate) primary_name: String,
    pub(crate) aliases: Vec<String>,
    pub(crate) kind: object_data::ObjectKind,
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
}

impl Object {
    /// Whether `name` matches this object's primary name or any alias.
    #[must_use]
    pub fn has_name(&self, name: &str) -> bool {
        self.primary_name == name || self.aliases.iter().any(|alias| alias == name)
    }

    /// Resolve `name` against `objects`, matching primary name or alias.
    #[must_use]
    pub fn resolve_by_name(objects: &[Object], name: &str) -> ObjectResolution {
        let mut matching: Vec<ObjectId> = objects
            .iter()
            .filter(|object| object.has_name(name))
            .map(|object| object.id.clone())
            .collect();

        match matching.len() {
            0 => ObjectResolution::NotFound,
            1 => ObjectResolution::Found(matching.remove(0)),
            _ => ObjectResolution::Ambiguous {
                ids: matching,
                alias: name.to_string(),
            },
        }
    }

    /// Build a live [`Object`] from authored
    /// [`ObjectData`](crate::data::object_data::ObjectData).
    pub(crate) fn from_data(object: &object_data::ObjectData) -> Self {
        let door = object.door.as_ref().and_then(|door_data| {
            Direction::parse(&door_data.direction).map(|direction| DoorState {
                direction,
                to: door_data.to.clone().into(),
                locked: door_data.locked,
            })
        });

        Object {
            id: object.id.clone(),
            primary_name: object.primary_name.clone(),
            aliases: object.aliases.clone(),
            kind: object.kind,
            door,
            extra: object.extra.clone(),
        }
    }
}

/// Public, plain-data *owned* snapshot of an object, handed to game rules so
/// they can decide behaviour without reaching into the engine's internals.
///
/// Owned rather than borrowed (unlike [`Npc`](crate::world::npc::Npc)'s
/// accessors) because an [`Object`] moves between containers (room, hidden
/// set, inventory) during ordinary gameplay, and effect handlers often need
/// its name after a `&mut WorldState` call that would invalidate a borrow.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectInfo {
    pub id: ObjectId,
    pub name: String,
    pub aliases: Vec<String>,
    pub kind: object_data::ObjectKind,
    pub door: Option<DoorInfo>,
    pub extra: HashMap<String, data::ExtraValue>,
}

/// Public, plain-data view of a door on a scene object.
#[derive(Debug, Clone, PartialEq)]
pub struct DoorInfo {
    pub direction: Direction,
    pub to: RoomId,
    pub locked: bool,
}

impl ObjectInfo {
    pub(crate) fn from_object(object: &Object) -> Self {
        ObjectInfo {
            id: object.id.clone(),
            name: object.primary_name.clone(),
            aliases: object.aliases.clone(),
            kind: object.kind,
            door: object.door.as_ref().map(|door| DoorInfo {
                direction: door.direction,
                to: door.to.clone(),
                locked: door.locked,
            }),
            extra: object.extra.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::door_data::DoorData;

    fn object(id: &str, primary_name: &str, aliases: Vec<&str>) -> Object {
        Object {
            id: ObjectId::new(id),
            primary_name: primary_name.to_string(),
            aliases: aliases.into_iter().map(String::from).collect(),
            kind: object_data::ObjectKind::Item,
            door: None,
            extra: HashMap::new(),
        }
    }

    #[test]
    fn has_name_matches_primary_name() {
        let sword = object("sword", "glowing sword", vec!["sword"]);
        assert!(sword.has_name("glowing sword"));
    }

    #[test]
    fn has_name_matches_any_alias() {
        let sword = object("sword", "glowing sword", vec!["sword", "blade"]);
        assert!(sword.has_name("sword"));
        assert!(sword.has_name("blade"));
    }

    #[test]
    fn has_name_rejects_unrelated_names() {
        let sword = object("sword", "glowing sword", vec!["sword"]);
        assert!(!sword.has_name("shield"));
    }

    #[test]
    fn resolve_by_name_not_found_when_nothing_matches() {
        let objects = vec![object("sword", "sword", vec![])];
        assert_eq!(
            Object::resolve_by_name(&objects, "shield"),
            ObjectResolution::NotFound
        );
    }

    #[test]
    fn resolve_by_name_finds_a_unique_match() {
        let objects = vec![
            object("sword", "sword", vec![]),
            object("shield", "shield", vec![]),
        ];
        assert_eq!(
            Object::resolve_by_name(&objects, "shield"),
            ObjectResolution::Found(ObjectId::new("shield"))
        );
    }

    #[test]
    fn resolve_by_name_reports_ambiguity_across_objects() {
        let objects = vec![
            object("iron-key", "iron key", vec!["key"]),
            object("brass-key", "brass key", vec!["key"]),
        ];
        assert_eq!(
            Object::resolve_by_name(&objects, "key"),
            ObjectResolution::Ambiguous {
                ids: vec![ObjectId::new("iron-key"), ObjectId::new("brass-key")],
                alias: "key".to_string(),
            }
        );
    }

    #[test]
    fn from_data_parses_a_valid_door_direction() {
        let data = object_data::ObjectData {
            id: ObjectId::new("door"),
            primary_name: "door".to_string(),
            aliases: Vec::new(),
            kind: object_data::ObjectKind::Scene,
            door: Some(DoorData {
                direction: "north".to_string(),
                to: "corridor".to_string(),
                locked: true,
            }),
            extra: HashMap::new(),
        };
        let object = Object::from_data(&data);
        let door = object.door.expect("door state");
        assert_eq!(door.direction, Direction::North);
        assert_eq!(door.to, RoomId::new("corridor"));
        assert!(door.locked);
    }

    #[test]
    fn from_data_drops_door_state_for_an_unparsable_direction() {
        let data = object_data::ObjectData {
            id: ObjectId::new("door"),
            primary_name: "door".to_string(),
            aliases: Vec::new(),
            kind: object_data::ObjectKind::Scene,
            door: Some(DoorData {
                direction: "sideways".to_string(),
                to: "corridor".to_string(),
                locked: false,
            }),
            extra: HashMap::new(),
        };
        let object = Object::from_data(&data);
        assert!(object.door.is_none());
    }

    #[test]
    fn object_info_from_object_mirrors_a_doorless_object() {
        let sword = object("sword", "sword", vec!["blade"]);
        let info = ObjectInfo::from_object(&sword);
        assert_eq!(info.id, ObjectId::new("sword"));
        assert_eq!(info.name, "sword");
        assert_eq!(info.aliases, vec!["blade".to_string()]);
        assert_eq!(info.kind, object_data::ObjectKind::Item);
        assert_eq!(info.door, None);
    }

    #[test]
    fn object_info_from_object_carries_door_state() {
        let mut door = object("door", "door", vec![]);
        door.kind = object_data::ObjectKind::Scene;
        door.door = Some(DoorState {
            direction: Direction::West,
            to: RoomId::new("cellar"),
            locked: true,
        });
        let info = ObjectInfo::from_object(&door);
        let door_info = info.door.expect("door info");
        assert_eq!(door_info.direction, Direction::West);
        assert_eq!(door_info.to, RoomId::new("cellar"));
        assert!(door_info.locked);
    }
}
