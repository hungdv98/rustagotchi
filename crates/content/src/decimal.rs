//! The one place authored decimals become simulation numbers.
//!
//! ADR 0006 bans floating point from the core. That ban would be worthless if
//! content arrived as `f64` and were converted afterwards — the rounding would
//! already have happened, off-platform and unrepeatably.
//!
//! So authored decimals are written as **strings** in RON:
//!
//! ```ron
//! decay_per_tick: "0.25",
//! ```
//!
//! and parsed with [`domain::units::Fixed`]'s integer parser. No `f64` exists
//! anywhere in the pipeline.
//!
//! The cost is real: designers type quotes they would rather not. It buys a
//! guarantee that a content pack means exactly the same thing on every machine,
//! which is the difference between the offline-equivalence test being possible
//! and being aspirational.

use std::fmt;

use domain::units::{Fixed, Milli};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// An authored decimal, stored exactly.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Decimal(Fixed);

impl Decimal {
    pub const ZERO: Self = Decimal(Fixed::ZERO);
    pub const ONE: Self = Decimal(Fixed::ONE);

    #[must_use]
    pub const fn from_fixed(value: Fixed) -> Self {
        Decimal(value)
    }

    #[must_use]
    pub const fn fixed(self) -> Fixed {
        self.0
    }

    /// Interprets the value as a percentage and converts it to the `Milli`
    /// scale, where `"100"` becomes a full bar.
    #[must_use]
    pub fn as_percent(self) -> Milli {
        self.0.apply(Milli(1_000))
    }

    #[must_use]
    pub const fn is_negative(self) -> bool {
        self.0.raw() < 0
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl Serialize for Decimal {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for Decimal {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        raw.parse::<Fixed>()
            .map(Decimal)
            .map_err(|error| serde::de::Error::custom(format!("invalid decimal {raw:?}: {error}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimals_parse_from_authored_strings() {
        let parsed: Decimal = ron::from_str("\"0.25\"").expect("parse");
        assert_eq!(parsed.fixed(), Fixed::from_ratio(1, 4));
    }

    #[test]
    fn whole_numbers_and_signs_are_accepted() {
        assert_eq!(
            ron::from_str::<Decimal>("\"2\"").unwrap().fixed(),
            Fixed::from_int(2)
        );
        assert_eq!(
            ron::from_str::<Decimal>("\"-0.5\"").unwrap().fixed(),
            Fixed::from_ratio(-1, 2)
        );
    }

    #[test]
    fn a_bare_ron_float_is_refused() {
        // The whole point: an author who writes 0.25 instead of "0.25" gets an
        // error rather than a silent trip through f64.
        assert!(ron::from_str::<Decimal>("0.25").is_err());
    }

    #[test]
    fn malformed_decimals_name_themselves_in_the_error() {
        let error = ron::from_str::<Decimal>("\"1.2.3\"")
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("1.2.3"),
            "the error should quote the offending text: {error}"
        );
    }

    #[test]
    fn percentages_map_onto_the_milli_scale() {
        assert_eq!(
            Decimal::from_fixed(Fixed::from_int(100)).as_percent(),
            Milli(100_000)
        );
        assert_eq!(
            Decimal::from_fixed(Fixed::from_ratio(1, 2)).as_percent(),
            Milli(500)
        );
    }

    #[test]
    fn decimals_round_trip_through_ron() {
        for text in ["0", "1", "0.5", "-2.25", "12.0625"] {
            let value: Decimal = ron::from_str(&format!("{text:?}")).expect("parse");
            let encoded = ron::to_string(&value).expect("serialize");
            let decoded: Decimal = ron::from_str(&encoded).expect("re-parse");
            assert_eq!(value, decoded, "round trip failed for {text}");
        }
    }
}
