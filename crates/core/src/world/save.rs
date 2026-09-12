//! Save/load: persisting and restoring only the *dynamic* slice of a running
//! game onto an already freshly-built [`WorldState`] — flags, room/inventory
//! object placement, objects discarded out of the world entirely, door lock
//! state, and fired triggers.
//!
//! Never the *static* authored content (`data_interactions`, `triggers`,
//! `object_templates`, NPC dialogue trees): that always comes from
//! `WorldData`, rebuilt fresh on every load, so a content patch between save
//! and load is picked up rather than shadowed by a stale copy baked into the
//! save.
//!
//! Deliberately also never in-progress dialogue (`dialogue_state`,
//! `active_npc`): a save never resumes the player mid-conversation. All
//! three front-end modalities dispatch dialogue as an immediate `Action` in
//! response to the player's last input (a `talk`/`choose` command, or a
//! point-and-click click); there is no modality that needs a *load* to land
//! back inside a conversation turn, and doing so would additionally let an
//! old save replay dialogue against an NPC's tree after a content patch
//! changed it out from under the save. `load` always starts with no active
//! conversation, full stop. See `crates/core/tests/save_load.rs` for the
//! full contract.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::keys::object_id::ObjectId;
use crate::keys::trigger_id::TriggerId;
use crate::world::{WorldState, object, room};

/// An error saving or loading a game's progress.
#[derive(Debug)]
pub enum SaveError {
    /// The save string could not be serialized or parsed.
    Yaml(serde_yaml_ng::Error),
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::Yaml(err) => write!(f, "failed to save or load game progress: {err}"),
        }
    }
}

impl std::error::Error for SaveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SaveError::Yaml(err) => Some(err),
        }
    }
}

/// A serializable snapshot of only the *dynamic* slice of a running game:
/// flags, room/inventory object placement, objects discarded out of the
/// world entirely, door lock state, and fired triggers. Deliberately no
/// dialogue progress — see the module doc comment.
///
/// Ordered maps/sets throughout, not `HashMap`/`HashSet`: `WorldState`'s
/// internal collections are hash-based (fine for that use — order is never
/// observed), but a save is serialized text, and hash iteration order isn't
/// stable across the independently-built collections two separate `save()`
/// calls construct. Sorting here keeps two saves of the same unchanged state
/// byte-identical (a diffable save file, and a save that never spuriously
/// looks "changed").
#[derive(Debug, Serialize, Deserialize)]
struct WorldSnapshot {
    flags: Vec<String>,
    current_room: room::RoomId,
    inventory: Vec<ObjectId>,
    rooms: BTreeMap<room::RoomId, RoomSnapshot>,
    locked_doors: BTreeSet<ObjectId>,
    /// Known object ids (every key of `object_templates` at save time) that
    /// are placed in neither a room nor inventory — most commonly a `discard`
    /// effect's target (`DataEffect::Discard`), consumed out of the game
    /// entirely. Without this bucket, `apply_snapshot` would have no record
    /// that these ids should stay gone: a freshly-built `WorldState` places
    /// every authored object back into its default room, and the room/
    /// inventory loops below only ever *add* objects they're told about, so
    /// a discarded object would silently reappear in its original room.
    /// `#[serde(default)]` so a save written before this field existed still
    /// loads (as "nothing was discarded", the correct reading of its
    /// absence).
    #[serde(default)]
    discarded: BTreeSet<ObjectId>,
    fired_triggers: BTreeSet<TriggerId>,
}

/// A single room's object placement within a [`WorldSnapshot`].
#[derive(Debug, Serialize, Deserialize)]
struct RoomSnapshot {
    visible: Vec<ObjectId>,
    hidden: Vec<ObjectId>,
}

/// Save/load: persisting and restoring only the dynamic slice of the game.
impl WorldState {
    fn snapshot(&self) -> WorldSnapshot {
        let rooms: BTreeMap<room::RoomId, RoomSnapshot> = self
            .rooms
            .iter()
            .map(|(room_id, room)| {
                let snapshot = RoomSnapshot {
                    visible: room.objects().iter().map(|o| o.id.clone()).collect(),
                    hidden: room.hidden_objects().iter().map(|o| o.id.clone()).collect(),
                };
                (room_id.clone(), snapshot)
            })
            .collect();

        let locked_doors = self
            .rooms
            .values()
            .flat_map(|room| room.objects().iter().chain(room.hidden_objects().iter()))
            .filter(|object| object.door.as_ref().is_some_and(|door| door.locked))
            .map(|object| object.id.clone())
            .collect();

        let inventory: Vec<ObjectId> = self.player.objects().iter().map(|o| o.id.clone()).collect();

        let placed: BTreeSet<&ObjectId> = rooms
            .values()
            .flat_map(|room| room.visible.iter().chain(room.hidden.iter()))
            .chain(inventory.iter())
            .collect();
        let discarded: BTreeSet<ObjectId> = self
            .object_templates
            .keys()
            .filter(|id| !placed.contains(id))
            .cloned()
            .collect();

        WorldSnapshot {
            flags: self.flags.clone(),
            current_room: self.current_room_id.clone(),
            inventory,
            rooms,
            locked_doors,
            discarded,
            fired_triggers: self.fired_triggers.iter().cloned().collect(),
        }
    }

    /// Remove the object with `id` from wherever it currently lives (the
    /// player's inventory, or any room's visible or hidden list). Falling
    /// that, materialise it fresh from `object_templates` — a `grant`-only
    /// object (never listed in any room's `visible_objects`/`hidden_objects`,
    /// only ever willed into inventory by a `Grant` effect, e.g. `toenail`
    /// in `data/npcs.yaml`) has no placed instance to find at all until now.
    /// `None` only for an id that isn't a template either: a content patch
    /// removed the object entirely.
    fn take_object_anywhere(&mut self, id: &ObjectId) -> Option<object::Object> {
        if let Some(object) = self.player.remove_object(id) {
            return Some(object);
        }
        for room in self.rooms.values_mut() {
            if let Some(object) = room.remove_object(id) {
                return Some(object);
            }
            if let Some(object) = room.remove_hidden_object(id) {
                return Some(object);
            }
        }
        self.object_templates.get(id).cloned()
    }

    /// Restore `snapshot`'s dynamic state onto this (freshly built)
    /// `WorldState`. An id the snapshot mentions that no longer exists (a
    /// content patch removed it) is silently skipped; an object the
    /// snapshot never mentions keeps the placement `from_data` just gave it
    /// (e.g. new content added since the save was made); an id in
    /// `snapshot.discarded` is removed from that default placement instead
    /// of relocated, so a consumed object stays gone. `dialogue_state` and
    /// `active_npc` are left exactly as `from_data` set them (empty, no
    /// active conversation) — never restored from a save, by design (see
    /// the module doc comment).
    fn apply_snapshot(&mut self, snapshot: WorldSnapshot) {
        self.flags = snapshot.flags;
        self.fired_triggers = snapshot.fired_triggers.into_iter().collect();

        for (room_id, room_snapshot) in snapshot.rooms {
            for id in &room_snapshot.visible {
                if let Some(object) = self.take_object_anywhere(id)
                    && let Some(room) = self.rooms.get_mut(&room_id)
                {
                    room.add_object(object);
                }
            }
            for id in &room_snapshot.hidden {
                if let Some(object) = self.take_object_anywhere(id)
                    && let Some(room) = self.rooms.get_mut(&room_id)
                {
                    room.add_hidden_object(object);
                }
            }
        }

        for id in &snapshot.inventory {
            if let Some(object) = self.take_object_anywhere(id) {
                self.player.add_object(object);
            }
        }

        // Pull every discarded id back out of wherever the fresh
        // `from_data` build placed it (its default room) and drop it —
        // deliberately not re-placed anywhere, so it stays gone.
        for id in &snapshot.discarded {
            self.take_object_anywhere(id);
        }

        for room in self.rooms.values_mut() {
            for object in room.objects_mut() {
                if let Some(door) = object.door.as_mut() {
                    door.locked = snapshot.locked_doors.contains(&object.id);
                }
            }
            for object in room.hidden_objects_mut() {
                if let Some(door) = object.door.as_mut() {
                    door.locked = snapshot.locked_doors.contains(&object.id);
                }
            }
        }

        if self.rooms.contains_key(&snapshot.current_room) {
            self.current_room_id = snapshot.current_room;
        }
    }

    /// Serialize the current game's progress (not the authored content) to
    /// a human-readable string.
    pub(crate) fn save(&self) -> Result<String, SaveError> {
        serde_yaml_ng::to_string(&self.snapshot()).map_err(SaveError::Yaml)
    }

    /// Parse `save` and restore its progress onto this (freshly built)
    /// `WorldState`.
    pub(crate) fn restore(&mut self, save: &str) -> Result<(), SaveError> {
        let snapshot: WorldSnapshot = serde_yaml_ng::from_str(save).map_err(SaveError::Yaml)?;
        self.apply_snapshot(snapshot);
        Ok(())
    }
}
