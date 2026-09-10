use crate::keys::define_id;

define_id!(
    /// Identifier for a world object. All interactables share one id space.
    ///
    /// The id is the object's stable symbolic `key` from the authored world
    /// data (e.g. `iron-key`); numeric ids never appear in authored YAML.
    ObjectId
);
