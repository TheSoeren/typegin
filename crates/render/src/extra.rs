//! Internal helpers for descending through a `typegin_core::ExtraValue`
//! tree. Every reader of this crate's `extra.gui.*` convention (see
//! `hotspot.rs`, `asset.rs`) needs to step into an `ExtraValue::Table` the
//! same way, so that one step lives here instead of being copied per reader.
//! Not part of this crate's public API — a specific `gui.*` key's reader
//! (`hotspot_rect`, `sprite_key`, ...) is the public surface, not this.

use std::collections::HashMap;

use typegin_core::ExtraValue;

/// View `value` as a table, or `None` if it's some other `ExtraValue`
/// variant.
pub(crate) fn as_table(value: &ExtraValue) -> Option<&HashMap<String, ExtraValue>> {
    match value {
        ExtraValue::Table(table) => Some(table),
        _ => None,
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub(crate) fn as_number(value: &ExtraValue) -> Option<f32> {
    match value {
        ExtraValue::Int(n) => Some(*n as f32),
        ExtraValue::Float(n) => Some(*n as f32),
        _ => None,
    }
}

/// View `value` as a string, or `None` if it's some other `ExtraValue`
/// variant.
pub(crate) fn as_string(value: &ExtraValue) -> Option<&str> {
    match value {
        ExtraValue::Str(s) => Some(s.as_str()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_table_views_a_table_variant() {
        let mut table = HashMap::new();
        table.insert("x".to_string(), ExtraValue::Int(1));
        let value = ExtraValue::Table(table.clone());
        assert_eq!(as_table(&value), Some(&table));
    }

    #[test]
    fn as_table_is_none_for_a_non_table_variant() {
        assert_eq!(as_table(&ExtraValue::Str("nope".to_string())), None);
    }
}
