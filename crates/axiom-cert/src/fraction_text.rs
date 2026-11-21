use num_bigint::BigInt;
use num_rational::BigRational;
use serde::{Deserialize, Serialize};

/// The largest number of decimal digits allowed in a numerator or
/// denominator, not counting a sign.
const MAX_DIGITS: usize = 64;

/// A fraction as it appears in a certificate: two decimal strings, in lowest
/// terms.
///
/// The denominator is always positive and unsigned. The numerator is unsigned
/// unless [`Sign::Allowed`] is passed: then a negative value is written as
/// `-` followed by the canonical digits of its magnitude, and `-0` is not
/// allowed, so every value has exactly one encoding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FractionText {
    num: String,
    den: String,
}

/// Whether a numerator may carry a leading `-`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Sign {
    /// Only non-negative values are allowed.
    Forbidden,
    /// Negative values are allowed, written with a leading `-`.
    Allowed,
}

impl FractionText {
    /// The canonical text of a fraction. `BigRational` keeps its value in
    /// lowest terms with a positive denominator, and `BigInt` writes a
    /// negative value as `-` followed by its digits, so the output is
    /// canonical by construction.
    pub(crate) fn from_fraction(value: &BigRational) -> Self {
        Self {
            num: value.numer().to_string(),
            den: value.denom().to_string(),
        }
    }

    /// Whether `value` can be written: its numerator and denominator each
    /// have at most [`MAX_DIGITS`] digits, not counting a sign.
    pub(crate) fn fits(value: &BigRational) -> bool {
        let digits = |number: &BigInt| number.magnitude().to_string().len();
        digits(value.numer()) <= MAX_DIGITS && digits(value.denom()) <= MAX_DIGITS
    }

    /// Parses the fraction, or returns `None` for every encoding other than
    /// the canonical one.
    pub(crate) fn to_fraction(&self, sign: Sign) -> Option<BigRational> {
        let numerator = match (sign, self.num.strip_prefix('-')) {
            (Sign::Allowed, Some(magnitude)) => -canonical_integer(magnitude, Zero::Forbidden)?,
            _ => canonical_integer(&self.num, Zero::Allowed)?,
        };
        let denominator = canonical_integer(&self.den, Zero::Forbidden)?;
        let value = BigRational::new(numerator.clone(), denominator.clone());
        (value.numer() == &numerator && value.denom() == &denominator).then_some(value)
    }
}

/// Whether zero is a valid value for an integer field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Zero {
    Allowed,
    Forbidden,
}

/// Parses `text` as a decimal integer with no sign, no leading zeros and at
/// most [`MAX_DIGITS`] digits, or returns `None`.
fn canonical_integer(text: &str, zero: Zero) -> Option<BigInt> {
    let digits_only = !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
    if !digits_only || text.len() > MAX_DIGITS {
        return None;
    }
    if text == "0" {
        return (zero == Zero::Allowed).then(|| BigInt::from(0));
    }
    if text.starts_with('0') {
        return None;
    }
    text.parse().ok()
}
