//! The Hoeffding sample-count decision, exactly over the rationals.
//!
//! For `n` independent samples in `[0, 1]` with mean `μ`, Hoeffding's
//! inequality gives `P(|mean - μ| ≥ ε) ≤ 2 exp(-2 n ε²)`. A sample count is
//! enough for failure probability `δ` when `2 exp(-2 n ε²) ≤ δ`, that is
//! when `exp(x) ≥ 2 / δ` for `x = 2 n ε²`. [`exp_lower_bound`] computes a
//! rational `L ≤ exp(x)`, and [`hoeffding_holds`] accepts when `2 ≤ δ L`.
//! Using a lower bound for `exp(x)`, an upper bound for `exp(-x)`, is the
//! direction that keeps the decision sound: it can only reject a count that
//! is enough, never accept one that is not. The Lean checker's
//! `checkHoeffding` runs the same computation step for step, and its
//! soundness theorem proves the real inequality whenever it accepts. The
//! decision lives here so that both the simulator and the certificate reader
//! use the one implementation.

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Zero};

/// The terms `Σ_{k < K} y^k / k!` of the series lower bound for `exp(y)`.
pub const EXP_SERIES_TERMS: u32 = 16;

/// The bits kept after the binary point when rounding a bound down.
pub const EXP_PRECISION_BITS: u32 = 128;

/// The largest argument the bound looks at: `exp(min(x, 256)) ≤ exp(x)`.
pub const EXP_ARGUMENT_CAP: u32 = 256;

/// The halving steps the argument reduction tries at most.
const HALVING_FUEL: u32 = 16;

fn integer(value: u64) -> BigRational {
    BigRational::from_integer(BigInt::from(value))
}

fn power_of_two(bits: u32) -> BigRational {
    BigRational::from_integer(BigInt::from(1) << bits)
}

/// `max(0, ⌊z · 2^P⌋ / 2^P)`: a lower bound of a non-negative `z` with `P`
/// bits after the binary point, so the numbers stay small.
fn round_down(z: &BigRational) -> BigRational {
    let scale = power_of_two(EXP_PRECISION_BITS);
    let rounded = (z * &scale).floor() / scale;
    if rounded < BigRational::zero() {
        BigRational::zero()
    } else {
        rounded
    }
}

/// `Σ_{k < K} y^k / k!`, which is at most `exp(y)` for `y ≥ 0`.
fn series(y: &BigRational) -> BigRational {
    let mut sum = BigRational::zero();
    let mut term = BigRational::one();
    for k in 0..EXP_SERIES_TERMS {
        sum += &term;
        term = term * y / integer(u64::from(k) + 1);
    }
    sum
}

/// The number of halvings that bring `x` to at most one, up to the fuel.
/// Soundness does not depend on this choice; it only keeps the series
/// accurate.
fn halvings(x: &BigRational) -> u32 {
    let mut current = x.clone();
    let mut count = 0;
    let two = integer(2);
    while count < HALVING_FUEL && current > BigRational::one() {
        current /= &two;
        count += 1;
    }
    count
}

/// A rational lower bound for `exp(x)`.
///
/// The argument is clamped to `[0, 256]`, reduced to `y = x / 2^s` with `s`
/// halvings, bounded below by the series `Σ_{k < 16} y^k / k!`, and squared
/// back `s` times, `exp(x) = exp(y)^(2^s)`. Every intermediate value is
/// rounded down to 128 binary places, and squaring is monotone on
/// non-negative numbers, so each step stays a lower bound.
#[must_use]
pub fn exp_lower_bound(x: &BigRational) -> BigRational {
    let cap = integer(u64::from(EXP_ARGUMENT_CAP));
    let clamped = if *x < BigRational::zero() {
        BigRational::zero()
    } else if *x > cap {
        cap
    } else {
        x.clone()
    };
    let steps = halvings(&clamped);
    let reduced = clamped / power_of_two(steps);
    let mut bound = round_down(&series(&reduced));
    for _ in 0..steps {
        bound = round_down(&(&bound * &bound));
    }
    bound
}

/// Whether `n` samples give tolerance `epsilon` with failure probability at
/// most `delta`: accepts only when `2 exp(-2 n ε²) ≤ δ` holds. A `delta` of
/// zero or below is always rejected, because the bound is non-negative and
/// so `δ L ≤ 0 < 2`; the Lean checker tests `0 < δ` explicitly for its proof.
#[must_use]
pub fn hoeffding_holds(n: u64, epsilon: &BigRational, delta: &BigRational) -> bool {
    let exponent = integer(2) * integer(n) * epsilon * epsilon;
    integer(2) <= delta * exp_lower_bound(&exponent)
}
