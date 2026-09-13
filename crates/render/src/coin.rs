//! Turning `GameEngine::verbs_for`'s result into a radial "verb coin"
//! layout: an ordered, angularly-spaced list a renderer can draw as a ring
//! of labels/icons around a hotspot.
//!
//! `verbs_for` returns a `HashSet<Verb>` — iteration order unspecified — so
//! a coin needs its own fixed convention for "which slot" a verb lands in.
//! That's this crate's concern, not core's: core doesn't order verbs
//! because it doesn't render them.

use std::collections::HashSet;
use std::f32::consts::TAU;

use typegin_core::Verb;

/// Stable slot ordering for every verb a coin can show.
const VERB_ORDER: [Verb; 7] = [
    Verb::Go,
    Verb::Talk,
    Verb::Take,
    Verb::Examine,
    Verb::Use,
    Verb::Drop,
    Verb::Look,
];

/// One entry in a laid-out verb coin: the verb, and the angle (radians, 0 =
/// first slot, increasing clockwise) its label/icon should sit at around
/// the hotspot's center.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoinEntry {
    pub verb: Verb,
    pub angle: f32,
}

/// Lay `verbs` out evenly around a circle, in [`VERB_ORDER`]. Empty input
/// yields an empty coin — the caller decides whether "no verbs" should even
/// be reachable (e.g. by not opening a coin at all when there's only one).
#[must_use]
#[allow(clippy::cast_precision_loss, clippy::implicit_hasher)]
pub fn layout(verbs: &HashSet<Verb>) -> Vec<CoinEntry> {
    let ordered: Vec<Verb> = VERB_ORDER
        .into_iter()
        .filter(|verb| verbs.contains(verb))
        .collect();
    let count = ordered.len();
    ordered
        .into_iter()
        .enumerate()
        .map(|(i, verb)| CoinEntry {
            verb,
            angle: (i as f32) * TAU / (count as f32),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_verbs_lay_out_to_an_empty_coin() {
        assert_eq!(layout(&HashSet::new()), Vec::new());
    }

    #[test]
    fn a_single_verb_sits_at_angle_zero() {
        let verbs = HashSet::from([Verb::Examine]);
        assert_eq!(
            layout(&verbs),
            vec![CoinEntry {
                verb: Verb::Examine,
                angle: 0.0
            }]
        );
    }

    #[test]
    fn multiple_verbs_are_spaced_evenly_in_a_fixed_order() {
        let verbs = HashSet::from([Verb::Examine, Verb::Go, Verb::Take]);
        let entries = layout(&verbs);
        assert_eq!(entries.len(), 3);
        // Fixed order (VERB_ORDER), not hash-set iteration order.
        assert_eq!(entries[0].verb, Verb::Go);
        assert_eq!(entries[1].verb, Verb::Take);
        assert_eq!(entries[2].verb, Verb::Examine);
        // Evenly spaced around the circle.
        let step = TAU / 3.0;
        assert!((entries[1].angle - step).abs() < 1e-5);
        assert!((entries[2].angle - 2.0 * step).abs() < 1e-5);
    }
}
