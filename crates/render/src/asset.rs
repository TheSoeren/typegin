//! The asset-provider extension point: how a specific game hands `render`
//! the textures/backgrounds/sprites it draws, without `render` ever knowing
//! about that game's actual content (see AGENTS.md's "Non-negotiable"
//! section - `render` stays a reusable engine, never a specific game).
//!
//! Mirrors `hotspot.rs`'s split: `render` owns a convention for finding an
//! opaque *key* inside an object's `extra` data (`extra.gui.sprite`, read by
//! [`sprite_key`]), and a specific game owns turning that key into an actual
//! drawable handle, by implementing [`AssetProvider`]. `render` never
//! interprets `AssetProvider::Handle` itself - it only ever hands the handle
//! back to the same game's own drawing code - so it can be a `macroquad`
//! `Texture2D`, an atlas index, or anything else a consuming game's asset
//! pipeline produces.

use std::collections::HashMap;

use typegin_core::ExtraValue;

use crate::extra::{as_string, as_table};

/// Implemented by a specific game to resolve an opaque sprite `key`
/// (authored per-object via `extra.gui.sprite`, see [`sprite_key`]) to a
/// drawable asset handle. `render` calls this at draw time; it never caches,
/// loads, or otherwise owns the underlying asset itself - that stays the
/// game's responsibility, same as it owns the `Handle` type.
pub trait AssetProvider {
    type Handle: Clone;

    /// Resolve `key` to a handle, or `None` if this provider has nothing
    /// for it (an unauthored or not-yet-loaded key) - a caller decides what
    /// a missing asset means (skip drawing, draw a placeholder, ...).
    fn asset(&self, key: &str) -> Option<Self::Handle>;
}

/// Convenience impl: a plain lookup table of already-loaded handles is
/// trivially an [`AssetProvider`], for a game that preloads everything up
/// front instead of resolving keys lazily.
#[allow(clippy::implicit_hasher)]
impl<H: Clone> AssetProvider for HashMap<String, H> {
    type Handle = H;

    fn asset(&self, key: &str) -> Option<H> {
        self.get(key).cloned()
    }
}

/// Read this crate's `gui.sprite` convention out of an object's `extra`
/// data. Returns `None` for anything not yet authored (missing `gui` key,
/// missing `sprite` key, or a non-string value) - same contract as
/// [`crate::hotspot::hotspot_rect`], never panics on incomplete content.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn sprite_key(extra: &HashMap<String, ExtraValue>) -> Option<&str> {
    let gui = as_table(extra.get("gui")?)?;
    as_string(gui.get("sprite")?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(pairs: Vec<(&str, ExtraValue)>) -> ExtraValue {
        ExtraValue::Table(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    #[test]
    fn sprite_key_reads_the_gui_sprite_convention() {
        let mut extra = HashMap::new();
        extra.insert(
            "gui".to_string(),
            table(vec![(
                "sprite",
                ExtraValue::Str("chair_default".to_string()),
            )]),
        );
        assert_eq!(sprite_key(&extra), Some("chair_default"));
    }

    #[test]
    fn sprite_key_is_none_when_not_authored() {
        assert_eq!(sprite_key(&HashMap::new()), None);
    }

    #[test]
    fn sprite_key_is_none_when_malformed() {
        let mut extra = HashMap::new();
        extra.insert(
            "gui".to_string(),
            table(vec![("sprite", ExtraValue::Int(42))]),
        );
        assert_eq!(sprite_key(&extra), None);
    }

    #[test]
    fn hashmap_provider_returns_the_handle_for_a_known_key() {
        let mut assets = HashMap::new();
        assets.insert("chair_default".to_string(), "chair.png");
        assert_eq!(assets.asset("chair_default"), Some("chair.png"));
    }

    #[test]
    fn hashmap_provider_returns_none_for_an_unknown_key() {
        let assets: HashMap<String, &str> = HashMap::new();
        assert_eq!(assets.asset("missing"), None);
    }
}
