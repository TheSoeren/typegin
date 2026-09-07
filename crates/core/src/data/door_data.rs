use serde::Deserialize;

/// Optional door data on a [`Scene`](ObjectKind::Scene) object.
///
/// A door is an ordinary object (id, names, `extra`) that additionally leads
/// somewhere: `direction` is the compass direction the door occupies in its
/// room, `to` the room it leads into. `locked` blocks traversal, `hidden` is
/// *not* stored here — hidden-ness is list membership (the door object lives in
/// the room's `hidden_objects` until revealed). `gated_by` optionally links the
/// door to the object that unlocks it; the unlock behaviour itself is a rule.
#[derive(Debug, Clone, Deserialize)]
pub struct DoorData {
    pub direction: String,
    pub to: String,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub gated_by: Option<String>,
}
