//! The engine functions covered by proof 3.
//!
//! Every function here is translated to Lean by Charon and Aeneas and proved
//! correct against the checker's semantics. They must stay inside the
//! translatable subset: rationals only through [`ExactRational`], no loops,
//! iterators, closures, traits, `unsafe` or foreign error types. The public
//! methods of [`crate::TwoStateChain`], [`crate::AffineUpdate`] and
//! [`crate::Probability`] delegate to them. `proof3.toml` at the repository
//! root lists them, and `scripts/checkProof3.sh` fails if one leaves the
//! subset.

use crate::TwoStateChainError;
use crate::exact_rational::ExactRational;

/// The complement `1 - p`.
pub(crate) fn complement(p: &ExactRational) -> ExactRational {
    ExactRational::one().sub(p)
}

/// The contraction factor `1 - a - b` of a two-state chain.
pub(crate) fn contraction_factor(a: &ExactRational, b: &ExactRational) -> ExactRational {
    complement(a).sub(b)
}

/// The steady state `a / (a + b)` of a two-state chain.
pub(crate) fn steady_state(
    a: &ExactRational,
    b: &ExactRational,
) -> Result<ExactRational, TwoStateChainError> {
    let total = a.add(b);
    if total.is_zero() {
        Err(TwoStateChainError::NoUniqueFixedPoint)
    } else {
        Ok(a.div(&total))
    }
}

/// One step `factor * value + offset` of an affine update.
pub(crate) fn affine_apply(
    factor: &ExactRational,
    offset: &ExactRational,
    value: &ExactRational,
) -> ExactRational {
    factor.mul(value).add(offset)
}
