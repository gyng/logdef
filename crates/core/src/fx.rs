//! Q8.8 fixed-point arithmetic.
//!
//! The simulation contains no floating point. Every sub-integer
//! quantity — positions between slots, rates per tick, terrain yield
//! multipliers — is an `Fx`: a signed Q8.8 value stored in an `i32`,
//! where `FX_ONE` (256) represents 1.0.
//!
//! Multiplication and division widen through `i64` and truncate toward
//! zero, so results are bit-identical on every target that has 64-bit
//! integers. That is the whole point: a run recorded on x86 replays on
//! ARM and in wasm with the same state hashes.
//!
//! `to_f32` exists only for serialising snapshots at the presentation
//! boundary. Nothing inside the simulation may call it.

use std::fmt;
use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

use serde::{Deserialize, Serialize};

/// Number of fractional bits in an `Fx`.
pub const FX_SHIFT: u32 = 8;

/// The `Fx` representation of 1.0.
pub const FX_ONE: i32 = 1 << FX_SHIFT;

/// Fixed-point scalar: Q8.8 over `i32`. `Fx(256)` is 1.0.
///
/// Serialises as its raw integer so replays and saves are exact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Fx(pub i32);

impl Fx {
    pub const ZERO: Fx = Fx(0);
    pub const ONE: Fx = Fx(FX_ONE);

    /// Exact whole number.
    #[inline]
    #[must_use]
    pub const fn from_int(value: i32) -> Self {
        Fx(value << FX_SHIFT)
    }

    /// `numerator / denominator`, truncated toward zero.
    ///
    /// The one sanctioned way to write a fractional constant in code:
    /// `Fx::ratio(1, 3)` rather than a float literal.
    #[inline]
    #[must_use]
    pub const fn ratio(numerator: i32, denominator: i32) -> Self {
        if denominator == 0 {
            return Fx(0);
        }
        Fx((((numerator as i64) << FX_SHIFT) / denominator as i64) as i32)
    }

    /// Whole part, truncated toward negative infinity.
    #[inline]
    #[must_use]
    pub const fn floor_int(self) -> i32 {
        self.0 >> FX_SHIFT
    }

    /// Fractional part in `[0, 1)`, as raw Q8.8 units.
    #[inline]
    #[must_use]
    pub const fn frac_raw(self) -> i32 {
        self.0 & (FX_ONE - 1)
    }

    #[inline]
    #[must_use]
    pub const fn abs(self) -> Self {
        Fx(self.0.abs())
    }

    #[inline]
    #[must_use]
    pub fn min(self, other: Self) -> Self {
        Fx(self.0.min(other.0))
    }

    #[inline]
    #[must_use]
    pub fn max(self, other: Self) -> Self {
        Fx(self.0.max(other.0))
    }

    #[inline]
    #[must_use]
    pub fn clamp(self, low: Self, high: Self) -> Self {
        Fx(self.0.clamp(low.0, high.0))
    }

    /// Presentation only — never call this inside a system.
    #[inline]
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn to_f32(self) -> f32 {
        self.0 as f32 / FX_ONE as f32
    }
}

impl Add for Fx {
    type Output = Fx;
    #[inline]
    fn add(self, rhs: Fx) -> Fx {
        Fx(self.0.saturating_add(rhs.0))
    }
}

impl Sub for Fx {
    type Output = Fx;
    #[inline]
    fn sub(self, rhs: Fx) -> Fx {
        Fx(self.0.saturating_sub(rhs.0))
    }
}

impl Neg for Fx {
    type Output = Fx;
    #[inline]
    fn neg(self) -> Fx {
        Fx(-self.0)
    }
}

impl Mul for Fx {
    type Output = Fx;
    /// Widens through `i64` and truncates toward zero.
    #[inline]
    fn mul(self, rhs: Fx) -> Fx {
        Fx(((self.0 as i64 * rhs.0 as i64) >> FX_SHIFT) as i32)
    }
}

impl Mul<i32> for Fx {
    type Output = Fx;
    #[inline]
    fn mul(self, rhs: i32) -> Fx {
        Fx(self.0.saturating_mul(rhs))
    }
}

impl Div for Fx {
    type Output = Fx;
    /// Widens through `i64` and truncates toward zero. Division by zero
    /// yields zero rather than panicking — a stalled rate, not a crash.
    #[inline]
    fn div(self, rhs: Fx) -> Fx {
        if rhs.0 == 0 {
            return Fx(0);
        }
        Fx((((self.0 as i64) << FX_SHIFT) / rhs.0 as i64) as i32)
    }
}

impl AddAssign for Fx {
    #[inline]
    fn add_assign(&mut self, rhs: Fx) {
        *self = *self + rhs;
    }
}

impl SubAssign for Fx {
    #[inline]
    fn sub_assign(&mut self, rhs: Fx) {
        *self = *self - rhs;
    }
}

impl fmt::Display for Fx {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Two decimal places, computed in integers.
        let whole = self.0 >> FX_SHIFT;
        let hundredths = ((self.0 & (FX_ONE - 1)) as i64 * 100) >> FX_SHIFT;
        write!(f, "{whole}.{hundredths:02}")
    }
}

/// A distance along the world, in Q8.8 **paces**, widened to `i64` so a
/// run can walk a very long way without wrapping.
pub type Paces = i64;

/// Promote an `Fx` rate into the `Paces` domain.
#[inline]
#[must_use]
pub const fn paces_from_fx(value: Fx) -> Paces {
    value.0 as i64
}

/// Whole paces, truncated toward negative infinity.
#[inline]
#[must_use]
pub const fn paces_to_int(value: Paces) -> i64 {
    value >> FX_SHIFT
}

/// Whole paces promoted into the Q8.8 `Paces` domain.
#[inline]
#[must_use]
pub const fn paces_from_int(value: i64) -> Paces {
    value << FX_SHIFT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_numbers_round_trip() {
        for n in -100..100 {
            assert_eq!(Fx::from_int(n).floor_int(), n);
        }
    }

    #[test]
    fn ratio_is_exact_for_powers_of_two() {
        assert_eq!(Fx::ratio(1, 2), Fx(128));
        assert_eq!(Fx::ratio(1, 4), Fx(64));
        assert_eq!(Fx::ratio(3, 4), Fx(192));
    }

    #[test]
    fn multiplication_matches_hand_computed_q88() {
        // 1.5 * 2.0 == 3.0
        assert_eq!(Fx(384) * Fx(512), Fx(768));
        // 0.5 * 0.5 == 0.25
        assert_eq!(Fx(128) * Fx(128), Fx(64));
    }

    #[test]
    fn division_by_zero_is_zero_not_panic() {
        assert_eq!(Fx::ONE / Fx::ZERO, Fx::ZERO);
        assert_eq!(Fx::ratio(1, 0), Fx::ZERO);
    }

    #[test]
    fn accumulation_never_drifts() {
        // Adding a fixed rate N times equals rate * N exactly. This is
        // the property the whole movement model leans on.
        let rate = Fx::ratio(1, 7);
        let mut acc = Fx::ZERO;
        for _ in 0..1000 {
            acc += rate;
        }
        assert_eq!(acc, Fx(rate.0 * 1000));
    }

    #[test]
    fn floor_and_frac_agree() {
        let value = Fx::from_int(3) + Fx::ratio(1, 4);
        assert_eq!(value.floor_int(), 3);
        assert_eq!(value.frac_raw(), 64);
    }

    #[test]
    fn display_is_integer_only() {
        assert_eq!(Fx::from_int(2).to_string(), "2.00");
        assert_eq!((Fx::from_int(1) + Fx::ratio(1, 2)).to_string(), "1.50");
    }
}
