//! Editing authored YAML content via real parsing: `serde_yaml_ng` (the
//! same crate `typegin_core` itself parses `WorldData` with) parses the
//! whole document into a generic [`serde_yaml_ng::Value`] tree, a function
//! here mutates the one value a caller asked to change, and the whole tree
//! is reserialized back out.
//!
//! This is a full round trip, not a surgical edit: comments are dropped,
//! and formatting (flow-style `{ x: 1, y: 2 }` mappings, integer-looking
//! numbers, key order, blank lines) is not guaranteed to survive — a
//! deliberate trade-off in favor of using a real, correct YAML parser
//! instead of ad hoc text scanning. A single content file like
//! `items.yaml` also isn't a complete, valid `WorldData` on its own (no
//! `rooms`/`interactions`/...), which is exactly why this operates on a
//! generic [`serde_yaml_ng::Value`] rather than `typegin_core::WorldData`
//! itself — that, and `WorldData` (along with `ExtraValue` and friends)
//! only derives `Deserialize`, not `Serialize`, so it can't be
//! reserialized without changes to `core` this crate has no need to make.

use render::hotspot::Rect;

/// Replace the `gui.hotspot` value on the object authored as
/// `- key: {object_key}` with `rect`, and return the whole document
/// reserialized.
///
/// Returns `None` if `yaml` doesn't parse, `object_key` isn't found among
/// the top-level `objects` list, or the object it names has no
/// `extra.gui.hotspot` to replace.
#[must_use]
pub fn set_hotspot(yaml: &str, object_key: &str, rect: Rect) -> Option<String> {
    todo!(
        "parse yaml into a serde_yaml_ng::Value, find the entry in the top-level `objects` \
         sequence whose `key` field equals object_key, replace its `extra.gui.hotspot` mapping \
         with rect's x/y/w/h (e.g. via serde_yaml_ng::to_value on a small helper struct or a \
         hand-built Mapping), then serde_yaml_ng::to_string the whole Value back out"
    )
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use typegin_core::ExtraValue;

    use super::*;

    const FIXTURE: &str = "\
objects:
  - key: chair
    primary_name: Chair
    extra:
      gui:
        hotspot: { x: 80, y: 370, w: 90, h: 140 }
        sprite: chair_default

  - key: table
    primary_name: Table
    extra:
      gui:
        hotspot: { x: 300, y: 385, w: 140, h: 110 }
        sprite: table_default
";

    /// Re-parse `yaml` generically and pull out `object_key`'s `extra`
    /// table as the same type `render::hotspot::hotspot_rect` reads —
    /// lets tests verify round-trip correctness through already-tested
    /// code, rather than asserting on `set_hotspot`'s exact output text
    /// (which full reserialization doesn't keep stable — see this
    /// module's doc comment).
    fn object_extra(yaml: &str, object_key: &str) -> HashMap<String, ExtraValue> {
        let root: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).expect("valid yaml");
        let objects = root["objects"].as_sequence().expect("an objects list");
        let object = objects
            .iter()
            .find(|object| object["key"].as_str() == Some(object_key))
            .expect("object present");
        serde_yaml_ng::from_value(object["extra"].clone()).expect("valid extra table")
    }

    #[test]
    fn set_hotspot_updates_the_matching_objects_hotspot_value() {
        let rect = Rect {
            x: 100.0,
            y: 370.0,
            w: 90.0,
            h: 140.0,
        };
        let updated = set_hotspot(FIXTURE, "chair", rect).expect("chair has a hotspot");

        let extra = object_extra(&updated, "chair");
        assert_eq!(render::hotspot::hotspot_rect(&extra), Some(rect));
    }

    #[test]
    fn set_hotspot_does_not_change_a_different_objects_hotspot() {
        let rect = Rect {
            x: 100.0,
            y: 370.0,
            w: 90.0,
            h: 140.0,
        };
        let updated = set_hotspot(FIXTURE, "chair", rect).expect("chair has a hotspot");

        let table_extra = object_extra(&updated, "table");
        assert_eq!(
            render::hotspot::hotspot_rect(&table_extra),
            Some(Rect {
                x: 300.0,
                y: 385.0,
                w: 140.0,
                h: 110.0,
            })
        );
    }

    #[test]
    fn set_hotspot_returns_none_when_the_yaml_is_malformed() {
        let rect = Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        };
        assert_eq!(set_hotspot("not: [valid", "chair", rect), None);
    }

    #[test]
    fn set_hotspot_returns_none_when_the_object_key_is_not_found() {
        let rect = Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        };
        assert_eq!(set_hotspot(FIXTURE, "nonexistent", rect), None);
    }

    #[test]
    fn set_hotspot_returns_none_when_the_object_has_no_hotspot() {
        let yaml = "\
objects:
  - key: chair-leg
    primary_name: Chair leg
    extra:
      description: The loose chair leg.
";
        let rect = Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        };
        assert_eq!(set_hotspot(yaml, "chair-leg", rect), None);
    }
}
