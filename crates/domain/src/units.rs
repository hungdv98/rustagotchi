//! Integer quantity types. See ADR 0006.
//!
//! Every simulated quantity in the game is one of these. Floating point is
//! banned in this crate and in `simulation` precisely so that
//! `assert_eq!(live, offline)` and `assert_eq!(saved, loaded)` can be exact
//! equality rather than a tolerance that has to be tuned and that hides
//! regressions below its threshold.
//!
//! # The two scales
//!
//! * [`Milli`] — a *quantity*, in thousandths of a unit. `Milli(1_000)` is one
//!   unit. Needs and stats are expressed as percentages on this scale, so a full
//!   bar is `Milli(100_000)` and reads as `100.000`.
//! * [`Fixed`] — a *rate or multiplier*, Q16.16 signed fixed point.
//!
//! # Rounding
//!
//! Every lossy operation truncates **toward zero**, matching Rust's integer
//! division. This is stated once, here, and tested once. Note that it is *not*
//! what an arithmetic right shift does: `>>` floors toward negative infinity, so
//! `Fixed` multiplication divides rather than shifts.
//!
//! Truncation is a design input, not an accident. A decay rate must be chosen so
//! that the per-tick result does not truncate to zero, or the effect silently
//! never happens. `Milli` resolution over the 0..=100_000 need scale gives ample
//! headroom: draining a full bar over seven days is 10 per tick.

use core::fmt;
use core::str::FromStr;

use serde::{Deserialize, Serialize};

/// A quantity in thousandths of a unit.
///
/// Needs and stats live on a percentage scale: [`Milli::ZERO`] is empty and
/// `Milli(100_000)` is full, displayed as `100.000`.
///
/// Arithmetic saturates rather than wrapping. A silent wrap in a save file is a
/// corrupted pet; a saturated one is merely a capped pet.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Milli(pub i32);

impl Milli {
    /// Thousandths per whole unit.
    pub const PER_UNIT: i32 = 1_000;

    pub const ZERO: Self = Milli(0);
    pub const ONE: Self = Milli(Self::PER_UNIT);

    /// `n` whole units. Saturates rather than overflowing.
    #[must_use]
    pub const fn from_units(units: i32) -> Self {
        Milli(units.saturating_mul(Self::PER_UNIT))
    }

    /// The whole-unit part, truncated toward zero.
    #[must_use]
    pub const fn whole_units(self) -> i32 {
        self.0 / Self::PER_UNIT
    }

    #[must_use]
    pub const fn saturating_add(self, rhs: Self) -> Self {
        Milli(self.0.saturating_add(rhs.0))
    }

    #[must_use]
    pub const fn saturating_sub(self, rhs: Self) -> Self {
        Milli(self.0.saturating_sub(rhs.0))
    }

    #[must_use]
    pub const fn saturating_neg(self) -> Self {
        Milli(self.0.saturating_neg())
    }

    /// Clamps into `lo..=hi`.
    ///
    /// # Panics
    ///
    /// If `lo > hi`.
    #[must_use]
    pub const fn clamp_to(self, lo: Self, hi: Self) -> Self {
        assert!(lo.0 <= hi.0, "clamp_to called with an inverted range");
        if self.0 < lo.0 {
            lo
        } else if self.0 > hi.0 {
            hi
        } else {
            self
        }
    }

    /// Multiplies by the rational `num / den`, truncating toward zero.
    ///
    /// Computed in `i64` and saturated back into `i32`, so intermediate values
    /// cannot overflow.
    ///
    /// # Panics
    ///
    /// If `den` is zero. Denominators originate in code or in content that the
    /// Phase 2 validator range-checks, so a zero here is a programming error and
    /// failing loudly beats producing silently wrong arithmetic.
    #[must_use]
    pub const fn scale(self, num: i32, den: i32) -> Self {
        assert!(den != 0, "Milli::scale called with a zero denominator");
        let scaled = (self.0 as i64 * num as i64) / den as i64;
        Milli(saturate_i32(scaled))
    }

    /// `self` as a fraction of `whole`, expressed in thousandths.
    ///
    /// `Milli(50_000).ratio_of(Milli(100_000))` is `Milli(500)` — that is, 0.5.
    /// Returns [`Milli::ZERO`] when `whole` is zero.
    #[must_use]
    pub const fn ratio_of(self, whole: Self) -> Self {
        if whole.0 == 0 {
            return Milli::ZERO;
        }
        let scaled = (self.0 as i64 * Self::PER_UNIT as i64) / whole.0 as i64;
        Milli(saturate_i32(scaled))
    }

    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    #[must_use]
    pub const fn is_negative(self) -> bool {
        self.0 < 0
    }
}

impl fmt::Display for Milli {
    /// Always three decimal places: `Milli(1_500)` renders as `1.500`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else { "" };
        // `unsigned_abs` so that i32::MIN does not overflow on negation.
        let magnitude = self.0.unsigned_abs();
        let units = magnitude / Milli::PER_UNIT as u32;
        let thousandths = magnitude % Milli::PER_UNIT as u32;
        write!(f, "{sign}{units}.{thousandths:03}")
    }
}

/// A signed Q16.16 fixed-point rate or multiplier.
///
/// Used for decay rates, stat modifiers and elemental effectiveness — anything
/// that scales a [`Milli`]. Multiplication is performed in `i128` and truncates
/// toward zero.
///
/// Serialization is the raw `i64`, which is exact. The decimal string form
/// ([`Display`](fmt::Display) / [`FromStr`]) is for authoring and diagnostics;
/// it is also exact, because 2^-16 terminates in decimal, but it is deliberately
/// not what saves store.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Fixed(i64);

impl Fixed {
    const FRAC_BITS: u32 = 16;
    const SCALE: i64 = 1 << Self::FRAC_BITS;

    /// 10^16 / 2^16 == 5^16, exactly. Every `Fixed` therefore has an exact
    /// decimal expansion of at most 16 fractional digits.
    const EXACT_FRAC_DIGITS: u32 = 16;
    const FRAC_TO_DECIMAL: i128 = 152_587_890_625; // 5^16

    pub const ZERO: Self = Fixed(0);
    pub const ONE: Self = Fixed(Self::SCALE);

    /// The raw Q16.16 representation. Prefer the arithmetic methods; this exists
    /// for serialization and tests.
    #[must_use]
    pub const fn raw(self) -> i64 {
        self.0
    }

    #[must_use]
    pub const fn from_raw(raw: i64) -> Self {
        Fixed(raw)
    }

    #[must_use]
    pub const fn from_int(value: i32) -> Self {
        Fixed(value as i64 * Self::SCALE)
    }

    /// The rational `num / den`, truncated toward zero.
    ///
    /// # Panics
    ///
    /// If `den` is zero.
    #[must_use]
    pub const fn from_ratio(num: i32, den: i32) -> Self {
        assert!(den != 0, "Fixed::from_ratio called with a zero denominator");
        Fixed((num as i64 * Self::SCALE) / den as i64)
    }

    /// Multiplies two rates, truncating toward zero. Saturates on overflow.
    #[must_use]
    pub const fn mul(self, rhs: Self) -> Self {
        let product = (self.0 as i128 * rhs.0 as i128) / Self::SCALE as i128;
        Fixed(saturate_i64(product))
    }

    /// Scales a quantity by this rate, truncating toward zero.
    #[must_use]
    pub const fn apply(self, value: Milli) -> Milli {
        let product = (value.0 as i128 * self.0 as i128) / Self::SCALE as i128;
        Milli(saturate_i32_from_i128(product))
    }

    #[must_use]
    pub const fn saturating_add(self, rhs: Self) -> Self {
        Fixed(self.0.saturating_add(rhs.0))
    }

    #[must_use]
    pub const fn saturating_sub(self, rhs: Self) -> Self {
        Fixed(self.0.saturating_sub(rhs.0))
    }

    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }
}

impl fmt::Display for Fixed {
    /// Exact decimal, with trailing zeros trimmed. `Fixed::ONE` renders as `1`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else { "" };
        let magnitude = self.0.unsigned_abs() as i128;
        let units = magnitude >> Fixed::FRAC_BITS;
        let frac_raw = magnitude & (Fixed::SCALE as i128 - 1);

        if frac_raw == 0 {
            return write!(f, "{sign}{units}");
        }

        // Exact: frac_raw / 2^16 == (frac_raw * 5^16) / 10^16.
        let decimal = frac_raw * Fixed::FRAC_TO_DECIMAL;
        let mut digits = format!(
            "{decimal:0width$}",
            width = Fixed::EXACT_FRAC_DIGITS as usize
        );
        while digits.ends_with('0') {
            digits.pop();
        }
        write!(f, "{sign}{units}.{digits}")
    }
}

/// Failure parsing a [`Fixed`] from its decimal form.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseFixedError {
    #[error("empty input")]
    Empty,
    #[error("invalid character {0:?}; expected an optional sign, digits, and at most one '.'")]
    InvalidCharacter(char),
    #[error("more than one decimal point")]
    MultipleDecimalPoints,
    #[error("{0} fractional digits; at most {max} are representable", max = 16)]
    TooManyFractionalDigits(usize),
    #[error("value does not fit in Q16.16")]
    OutOfRange,
}

impl FromStr for Fixed {
    type Err = ParseFixedError;

    /// Parses decimal notation with integer arithmetic only — no float ever
    /// appears, which is the point of ADR 0006. This is the single conversion
    /// used when authored content becomes simulation data.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err(ParseFixedError::Empty);
        }

        let (negative, body) = match s.as_bytes()[0] {
            b'-' => (true, &s[1..]),
            b'+' => (false, &s[1..]),
            _ => (false, s),
        };
        if body.is_empty() {
            return Err(ParseFixedError::Empty);
        }

        let mut parts = body.split('.');
        let int_part = parts.next().unwrap_or("");
        let frac_part = parts.next().unwrap_or("");
        if parts.next().is_some() {
            return Err(ParseFixedError::MultipleDecimalPoints);
        }
        if int_part.is_empty() && frac_part.is_empty() {
            return Err(ParseFixedError::Empty);
        }
        for c in body.chars() {
            if c != '.' && !c.is_ascii_digit() {
                return Err(ParseFixedError::InvalidCharacter(c));
            }
        }
        if frac_part.len() > Fixed::EXACT_FRAC_DIGITS as usize {
            return Err(ParseFixedError::TooManyFractionalDigits(frac_part.len()));
        }

        let units: i128 = if int_part.is_empty() {
            0
        } else {
            int_part
                .parse::<i128>()
                .map_err(|_| ParseFixedError::OutOfRange)?
        };

        let mut raw = units
            .checked_mul(Fixed::SCALE as i128)
            .ok_or(ParseFixedError::OutOfRange)?;

        if !frac_part.is_empty() {
            let frac: i128 = frac_part
                .parse::<i128>()
                .map_err(|_| ParseFixedError::OutOfRange)?;
            let denominator = 10i128.pow(frac_part.len() as u32);
            // Truncates toward zero, as ADR 0006 requires.
            raw += (frac * Fixed::SCALE as i128) / denominator;
        }

        if negative {
            raw = -raw;
        }
        if raw > i64::MAX as i128 || raw < i64::MIN as i128 {
            return Err(ParseFixedError::OutOfRange);
        }
        Ok(Fixed(raw as i64))
    }
}

const fn saturate_i32(value: i64) -> i32 {
    if value > i32::MAX as i64 {
        i32::MAX
    } else if value < i32::MIN as i64 {
        i32::MIN
    } else {
        value as i32
    }
}

const fn saturate_i32_from_i128(value: i128) -> i32 {
    if value > i32::MAX as i128 {
        i32::MAX
    } else if value < i32::MIN as i128 {
        i32::MIN
    } else {
        value as i32
    }
}

const fn saturate_i64(value: i128) -> i64 {
    if value > i64::MAX as i128 {
        i64::MAX
    } else if value < i64::MIN as i128 {
        i64::MIN
    } else {
        value as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn milli_displays_three_decimals() {
        assert_eq!(Milli(1_500).to_string(), "1.500");
        assert_eq!(Milli(100_000).to_string(), "100.000");
        assert_eq!(Milli(7).to_string(), "0.007");
        assert_eq!(Milli(-1_500).to_string(), "-1.500");
        assert_eq!(Milli::ZERO.to_string(), "0.000");
    }

    #[test]
    fn milli_display_survives_i32_min() {
        // Naive negation would overflow here.
        assert_eq!(Milli(i32::MIN).to_string(), "-2147483.648");
    }

    #[test]
    fn milli_arithmetic_saturates_instead_of_wrapping() {
        assert_eq!(Milli(i32::MAX).saturating_add(Milli::ONE), Milli(i32::MAX));
        assert_eq!(Milli(i32::MIN).saturating_sub(Milli::ONE), Milli(i32::MIN));
        assert_eq!(Milli::from_units(i32::MAX), Milli(i32::MAX));
    }

    #[test]
    fn scale_truncates_toward_zero_in_both_directions() {
        // 7/2 == 3.5 -> 3, and -7/2 == -3.5 -> -3, not -4.
        assert_eq!(Milli(7).scale(1, 2), Milli(3));
        assert_eq!(Milli(-7).scale(1, 2), Milli(-3));
    }

    #[test]
    fn scale_cannot_overflow_intermediates() {
        // i32 arithmetic would wrap; the i64 intermediate does not.
        assert_eq!(Milli(i32::MAX).scale(2, 2), Milli(i32::MAX));
        assert_eq!(Milli(i32::MAX).scale(2, 1), Milli(i32::MAX));
    }

    #[test]
    fn ratio_of_reports_thousandths() {
        assert_eq!(Milli(50_000).ratio_of(Milli(100_000)), Milli(500));
        assert_eq!(Milli(100_000).ratio_of(Milli(100_000)), Milli::ONE);
        assert_eq!(Milli(0).ratio_of(Milli(100_000)), Milli::ZERO);
    }

    #[test]
    fn ratio_of_zero_whole_is_zero_not_a_panic() {
        assert_eq!(Milli(42).ratio_of(Milli::ZERO), Milli::ZERO);
    }

    #[test]
    fn fixed_multiplication_truncates_toward_zero() {
        let half = Fixed::from_ratio(1, 2);
        assert_eq!(half.apply(Milli(7)), Milli(3));
        assert_eq!(half.apply(Milli(-7)), Milli(-3));
    }

    #[test]
    fn fixed_multiplication_is_not_an_arithmetic_shift() {
        // `>>` would floor to -1 here; truncation toward zero gives 0.
        let tiny = Fixed::from_raw(-1);
        assert_eq!(tiny.mul(Fixed::ONE), Fixed::from_raw(-1));
        assert_eq!(Fixed::from_ratio(-1, 3).apply(Milli(1)), Milli::ZERO);
    }

    #[test]
    fn fixed_identity_and_zero() {
        let rate = Fixed::from_ratio(3, 4);
        assert_eq!(rate.mul(Fixed::ONE), rate);
        assert_eq!(rate.mul(Fixed::ZERO), Fixed::ZERO);
        assert_eq!(Fixed::ONE.apply(Milli(12_345)), Milli(12_345));
    }

    #[test]
    fn fixed_display_is_exact() {
        assert_eq!(Fixed::ONE.to_string(), "1");
        assert_eq!(Fixed::ZERO.to_string(), "0");
        assert_eq!(Fixed::from_ratio(1, 2).to_string(), "0.5");
        assert_eq!(Fixed::from_ratio(-1, 2).to_string(), "-0.5");
        assert_eq!(Fixed::from_int(3).to_string(), "3");
        // The smallest representable step, written out in full.
        assert_eq!(Fixed::from_raw(1).to_string(), "0.0000152587890625");
    }

    #[test]
    fn fixed_parses_authored_decimals() {
        assert_eq!("1".parse::<Fixed>().unwrap(), Fixed::ONE);
        assert_eq!("0.5".parse::<Fixed>().unwrap(), Fixed::from_ratio(1, 2));
        assert_eq!("-0.5".parse::<Fixed>().unwrap(), Fixed::from_ratio(-1, 2));
        assert_eq!(".5".parse::<Fixed>().unwrap(), Fixed::from_ratio(1, 2));
        assert_eq!("+2.25".parse::<Fixed>().unwrap(), Fixed::from_ratio(9, 4));
        assert_eq!("  1.5  ".parse::<Fixed>().unwrap(), Fixed::from_ratio(3, 2));
    }

    #[test]
    fn fixed_rejects_malformed_input_rather_than_guessing() {
        assert!("".parse::<Fixed>().is_err());
        assert!("-".parse::<Fixed>().is_err());
        assert!("1.2.3".parse::<Fixed>().is_err());
        assert!("1e5".parse::<Fixed>().is_err());
        assert!("abc".parse::<Fixed>().is_err());
        assert!("0.00000000000000001".parse::<Fixed>().is_err()); // 17 digits
    }

    #[test]
    fn fixed_round_trips_through_its_decimal_form() {
        for raw in [0i64, 1, -1, 65_536, -65_536, 12_345, -98_765, 1 << 30] {
            let value = Fixed::from_raw(raw);
            let text = value.to_string();
            assert_eq!(
                text.parse::<Fixed>().unwrap(),
                value,
                "round trip failed for {text}"
            );
        }
    }

    #[test]
    fn clamp_to_bounds() {
        let lo = Milli::ZERO;
        let hi = Milli(100_000);
        assert_eq!(Milli(-5).clamp_to(lo, hi), lo);
        assert_eq!(Milli(200_000).clamp_to(lo, hi), hi);
        assert_eq!(Milli(50).clamp_to(lo, hi), Milli(50));
    }

    #[test]
    #[should_panic(expected = "inverted range")]
    fn clamp_to_rejects_an_inverted_range() {
        let _ = Milli(1).clamp_to(Milli(10), Milli(0));
    }

    #[test]
    #[should_panic(expected = "zero denominator")]
    fn scale_rejects_a_zero_denominator() {
        let _ = Milli(1).scale(1, 0);
    }
}
