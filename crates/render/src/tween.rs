//! A generic value that chases a target over time — the presentational
//! state a render loop mutates every frame, kept separate from
//! `typegin_core::WorldState` (the only source of truth for what's actually
//! true). `WorldState`/`Event` decide *what* changed; a [`Tween`] only ever
//! decides *how far along showing it* is, and never feeds back into game
//! state.

use getset::CopyGetters;

/// Anything a [`Tween`] can interpolate between.
pub trait Lerp: Copy {
    #[must_use]
    fn lerp(self, other: Self, t: f32) -> Self;
}

impl Lerp for f32 {
    fn lerp(self, other: Self, t: f32) -> Self {
        self + (other - self) * t
    }
}

impl Lerp for (f32, f32) {
    fn lerp(self, other: Self, t: f32) -> Self {
        (self.0.lerp(other.0, t), self.1.lerp(other.1, t))
    }
}

/// A value that animates linearly toward a target over `duration` seconds.
#[derive(Debug, Clone, Copy, PartialEq, CopyGetters)]
pub struct Tween<T: Lerp> {
    /// The value as of the last [`Tween::advance`] call.
    #[getset(get_copy = "pub")]
    current: T,
    start: T,
    target: T,
    elapsed: f32,
    duration: f32,
}

impl<T: Lerp> Tween<T> {
    /// A tween that starts already settled at `value`.
    #[must_use]
    pub fn settled(value: T) -> Self {
        Tween {
            current: value,
            start: value,
            target: value,
            elapsed: 0.0,
            duration: 0.0,
        }
    }

    /// Whether the tween has reached its target.
    #[must_use]
    pub fn is_settled(&self) -> bool {
        self.duration <= 0.0 || self.elapsed >= self.duration
    }

    /// Retarget toward `target`, animating over `duration` seconds starting
    /// from wherever the tween currently is (never from its old target),
    /// so retargeting mid-flight never causes a jump.
    pub fn set_target(&mut self, target: T, duration: f32) {
        self.start = self.current;
        self.target = target;
        self.elapsed = 0.0;
        self.duration = duration;
    }

    /// Advance the tween by `dt` seconds, updating [`Tween::current`].
    pub fn advance(&mut self, dt: f32) {
        if self.is_settled() {
            return;
        }
        self.elapsed = (self.elapsed + dt).min(self.duration);
        let t = self.elapsed / self.duration;
        self.current = self.start.lerp(self.target, t);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn settled_tween_is_settled_and_holds_its_value() {
        let tween = Tween::settled(5.0_f32);
        assert!(tween.is_settled());
        assert_eq!(tween.current(), 5.0);
    }

    #[test]
    fn advance_interpolates_toward_the_target() {
        let mut tween = Tween::settled(0.0_f32);
        tween.set_target(10.0, 2.0);
        tween.advance(1.0);
        assert_eq!(tween.current(), 5.0);
        assert!(!tween.is_settled());
        tween.advance(1.0);
        assert_eq!(tween.current(), 10.0);
        assert!(tween.is_settled());
    }

    #[test]
    fn advance_never_overshoots_the_target() {
        let mut tween = Tween::settled(0.0_f32);
        tween.set_target(10.0, 1.0);
        tween.advance(5.0);
        assert_eq!(tween.current(), 10.0);
        assert!(tween.is_settled());
    }

    #[test]
    fn retargeting_mid_flight_starts_from_the_current_value_not_the_old_target() {
        let mut tween = Tween::settled(0.0_f32);
        tween.set_target(10.0, 2.0);
        tween.advance(1.0);
        assert_eq!(tween.current(), 5.0);

        // Retarget before settling: the new animation must start at 5.0
        // (where it visually is), not jump back to 0.0 or forward to 10.0.
        tween.set_target(20.0, 1.0);
        assert_eq!(tween.current(), 5.0);
        tween.advance(1.0);
        assert_eq!(tween.current(), 20.0);
    }

    #[test]
    fn tuple_lerp_interpolates_both_components() {
        let mut tween = Tween::settled((0.0_f32, 0.0_f32));
        tween.set_target((10.0, -10.0), 1.0);
        tween.advance(0.5);
        assert_eq!(tween.current(), (5.0, -5.0));
    }
}
