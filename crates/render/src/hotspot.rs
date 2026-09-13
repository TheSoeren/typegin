//! Reading this crate's `gui.*` convention out of an object's opaque
//! `extra` data (see `typegin_core::ExtraValue` — core never interprets
//! these keys itself; every front-end owns its own convention on top of
//! them). `render`'s convention: `extra.gui.hotspot = {x, y, w, h}`.
//!
//! Bounding box is the starting shape for every hotspot; a specific object
//! can upgrade to a polygon or a pixel mask later without this type, or
//! anything that reads it, needing to change — hit-testing was never
//! coupled to how an object is actually drawn.

use std::collections::HashMap;

use typegin_core::ExtraValue;

/// An axis-aligned clickable region, in room-local coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    #[must_use]
    pub fn contains(self, point: (f32, f32)) -> bool {
        let (px, py) = point;
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

/// Read this crate's `gui.hotspot` convention out of an object's `extra`
/// data. Returns `None` for anything not yet authored (missing `gui` key,
/// missing `hotspot` key, or a malformed shape) — a caller decides what a
/// missing hotspot means (skip the object, draw a placeholder, ...); this
/// function never panics on incomplete content.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn hotspot_rect(extra: &HashMap<String, ExtraValue>) -> Option<Rect> {
    let gui = as_table(extra.get("gui")?)?;
    let hotspot = as_table(gui.get("hotspot")?)?;
    Some(Rect {
        x: as_number(hotspot.get("x")?)?,
        y: as_number(hotspot.get("y")?)?,
        w: as_number(hotspot.get("w")?)?,
        h: as_number(hotspot.get("h")?)?,
    })
}

fn as_table(value: &ExtraValue) -> Option<&HashMap<String, ExtraValue>> {
    match value {
        ExtraValue::Table(table) => Some(table),
        _ => None,
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn as_number(value: &ExtraValue) -> Option<f32> {
    match value {
        ExtraValue::Int(n) => Some(*n as f32),
        ExtraValue::Float(n) => Some(*n as f32),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(pairs: Vec<(&str, ExtraValue)>) -> ExtraValue {
        ExtraValue::Table(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    #[test]
    fn contains_is_true_inside_and_false_outside() {
        let rect = Rect {
            x: 10.0,
            y: 10.0,
            w: 20.0,
            h: 20.0,
        };
        assert!(rect.contains((15.0, 15.0)));
        assert!(!rect.contains((5.0, 5.0)));
        assert!(!rect.contains((30.0, 15.0)));
    }

    #[test]
    fn hotspot_rect_reads_the_gui_hotspot_convention() {
        let mut extra = HashMap::new();
        extra.insert(
            "gui".to_string(),
            table(vec![(
                "hotspot",
                table(vec![
                    ("x", ExtraValue::Int(10)),
                    ("y", ExtraValue::Int(20)),
                    ("w", ExtraValue::Float(100.0)),
                    ("h", ExtraValue::Int(50)),
                ]),
            )]),
        );
        assert_eq!(
            hotspot_rect(&extra),
            Some(Rect {
                x: 10.0,
                y: 20.0,
                w: 100.0,
                h: 50.0
            })
        );
    }

    #[test]
    fn hotspot_rect_is_none_when_not_authored() {
        assert_eq!(hotspot_rect(&HashMap::new()), None);
    }

    #[test]
    fn hotspot_rect_is_none_when_malformed() {
        let mut extra = HashMap::new();
        extra.insert(
            "gui".to_string(),
            ExtraValue::Str("not a table".to_string()),
        );
        assert_eq!(hotspot_rect(&extra), None);
    }
}
