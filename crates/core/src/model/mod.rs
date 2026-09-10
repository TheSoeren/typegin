pub mod dialogue_node_id;
pub mod dialogue_option_id;
pub mod npc_id;
pub mod object_id;
pub mod room_id;

/// Defines a symbolic-string newtype id: `new`/`get`/`into_key`, `Display`,
/// and conversions to/from `String`.
///
/// Every world-data id (object, room, npc, dialogue node, dialogue option)
/// shares this exact shape — the key is the id's stable symbolic name from
/// authored world data (e.g. `iron-key`), never a numeric surrogate.
macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, ::serde::Deserialize)]
        pub struct $name(String);

        impl $name {
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                $name(value.into())
            }

            /// The key as a string slice.
            #[must_use]
            pub fn get(&self) -> &str {
                &self.0
            }

            /// Consume the id, returning the key.
            #[must_use]
            pub fn into_key(self) -> String {
                self.0
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                $name::new(value)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                $name(value)
            }
        }

        impl From<$name> for String {
            fn from(id: $name) -> Self {
                id.0
            }
        }
    };
}

pub(crate) use define_id;

/// Outcome of resolving a player-typed noun against a set of named things.
///
/// Shared by every "resolve this name" lookup in the engine — world objects
/// ([`ObjectId`](crate::model::object_id::ObjectId)) and use-with targets
/// ([`Target`](crate::interaction::Target)) alike — so the three cases
/// (found, matched more than one name, matched none) are defined once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution<T> {
    Found(T),
    Ambiguous { ids: Vec<T>, alias: String },
    NotFound,
}
