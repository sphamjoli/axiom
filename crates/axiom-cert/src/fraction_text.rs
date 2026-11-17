use num_bigint::BigInt;
use num_rational::BigRational;
use serde::{Deserialize, Serialize};

use crate::CertificateError;

/// The largest number of decimal digits allowed in a numerator or
/// denominator.
const MAX_DIGITS: usize = 64;

/// A non-negative fraction as it appears in a certificate: two decimal
/// strings, in lowest terms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FractionText {
    num: String,
    den: String,
}

impl FractionText {
    /// The canonical text of a non-negative fraction. `BigRational` keeps its
    /// value in lowest terms with a positive denominator, so the output is
    /// canonical by construction.
    pub(crate) fn from_fraction(value: &BigRational) -> Self {
        Self {
            num: value.numer().to_string(),
            den: value.denom().to_string(),
        }
    }

    /// Parses the fraction at `row`, `entry`, rejecting every encoding other
    /// than the canonical one.
    pub(crate) fn to_fraction(
        &self,
        row: usize,
        entry: usize,
    ) -> Result<BigRational, CertificateError> {
        let non_canonical = CertificateError::NonCanonicalFraction { row, entry };
        let numerator = canonical_integer(&self.num, Zero::Allowed);
        let denominator = canonical_integer(&self.den, Zero::Forbidden);
        let (Some(numerator), Some(denominator)) = (numerator, denominator) else {
            return Err(non_canonical);
        };
        let value = BigRational::new(numerator.clone(), denominator.clone());
        if value.numer() != &numerator || value.denom() != &denominator {
            return Err(non_canonical);
        }
        Ok(value)
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
