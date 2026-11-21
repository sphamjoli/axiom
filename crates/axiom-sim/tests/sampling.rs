//! The generator, exact thresholds and seeded sampling of chains and
//! retries.

mod common;

use axiom_core::Probability;
use axiom_sim::{
    SimError, SplitMix64, Threshold, chain_matrix, required_samples, retry_probability,
    sample_chain, sample_retry,
};
use common::{TestError, ensure, links, ratio, retry};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::Signed;

/// Steps per trajectory. The links chain contracts by `λ = 3/5` per step, so
/// after 60 steps the distance to `d* = 1/4` is `(3/5)^60 / 4 < 10^-13`.
const STEPS: usize = 60;

#[test]
fn splitmix64_matches_the_reference_outputs_for_seed_zero() -> Result<(), TestError> {
    let mut rng = SplitMix64::new(0);
    let outputs = [rng.next_u64(), rng.next_u64(), rng.next_u64()];
    ensure(
        outputs
            == [
                0xE220_A839_7B1D_CDAF,
                0x6E78_9E6A_A1B9_65F4,
                0x06C4_5D18_8009_454F,
            ],
        format!("unexpected outputs {outputs:x?}"),
    )
}

#[test]
fn splitmix64_matches_the_reference_output_for_a_nonzero_seed() -> Result<(), TestError> {
    let mut rng = SplitMix64::new(1_234_567);
    let outputs = [rng.next_u64(), rng.next_u64()];
    ensure(
        outputs == [6_457_827_717_110_365_317, 3_203_168_211_198_807_973],
        format!("unexpected outputs {outputs:?}"),
    )
}

#[test]
fn thresholds_cut_exactly() -> Result<(), TestError> {
    let half = Threshold::of(&Probability::try_from_fraction(1, 2)?);
    ensure(half.admits((1 << 63) - 1), "1/2 admits 2^63 - 1")?;
    ensure(!half.admits(1 << 63), "1/2 rejects 2^63")?;
    let one = Threshold::of(&Probability::try_from_fraction(1, 1)?);
    ensure(one.admits(u64::MAX), "1 admits every draw")?;
    let zero = Threshold::of(&Probability::try_from_fraction(0, 1)?);
    ensure(!zero.admits(0), "0 admits no draw")?;
    // ⌈2^64 / 3⌉ = 6148914691236517206.
    let third = Threshold::of(&Probability::try_from_fraction(1, 3)?);
    ensure(
        third.admits(6_148_914_691_236_517_205),
        "1/3 admits its last draw",
    )?;
    ensure(
        !third.admits(6_148_914_691_236_517_206),
        "1/3 rejects the next draw",
    )
}

#[test]
fn chain_samples_are_reproducible_from_the_seed() -> Result<(), TestError> {
    let chain = links()?;
    let matrix = chain_matrix(&chain)?;
    let first = sample_chain(matrix, 0, 20, 200, 7)?;
    let again = sample_chain(matrix, 0, 20, 200, 7)?;
    let other = sample_chain(matrix, 0, 20, 200, 8)?;
    ensure(first == again, "same seed, same samples")?;
    ensure(first != other, "different seed, different samples")
}

#[test]
fn retry_samples_are_reproducible_from_the_seed() -> Result<(), TestError> {
    let model = retry()?;
    let success = retry_probability(&model)?;
    let first = sample_retry(success, 100, 200, 7);
    ensure(first == sample_retry(success, 100, 200, 7), "same seed")?;
    ensure(
        first != sample_retry(success, 100, 200, 8),
        "different seed",
    )
}

#[test]
fn out_of_range_start_is_refused() -> Result<(), TestError> {
    let chain = links()?;
    let result = sample_chain(chain_matrix(&chain)?, 2, 1, 1, 0);
    ensure(
        result == Err(SimError::StateOutOfRange(2)),
        "state 2 is refused",
    )
}

#[test]
fn parametric_models_are_refused() -> Result<(), TestError> {
    let chain = "param a in ]0, 1[\nchain c {\n  states x, y\n  from x: x 1 - a, y a\n  from y: x 1/2, y 1/2\n}\n";
    match axiom_model::parse(chain)? {
        axiom_model::CheckedModel::Chain(chain) => ensure(
            chain_matrix(&chain).err() == Some(SimError::Parametric),
            "a chain with parameters is refused",
        )?,
        axiom_model::CheckedModel::Retry(_) => return Err(TestError::Form),
    }
    let retry = "param p in ]0, 1[\nretry r {\n  success p\n}\n";
    match axiom_model::parse(retry)? {
        axiom_model::CheckedModel::Retry(retry) => ensure(
            retry_probability(&retry).err() == Some(SimError::Parametric),
            "a retry with parameters is refused",
        ),
        axiom_model::CheckedModel::Chain(_) => Err(TestError::Form),
    }
}

#[test]
fn zero_steps_end_where_they_start() -> Result<(), TestError> {
    let chain = links()?;
    let run = sample_chain(chain_matrix(&chain)?, 1, 0, 5, 7)?;
    ensure(
        run.final_states == vec![1; 5],
        "every trajectory stays in down",
    )
}

#[test]
fn zero_samples_give_empty_runs() -> Result<(), TestError> {
    let chain = links()?;
    let run = sample_chain(chain_matrix(&chain)?, 0, 10, 0, 7)?;
    ensure(run.samples() == 0 && run.count(0) == 0, "no trajectories")?;
    let model = retry()?;
    let retries = sample_retry(retry_probability(&model)?, 10, 0, 7);
    ensure(
        retries.samples() == 0 && retries.total_attempts() == 0,
        "no runs",
    )
}

#[test]
fn zero_attempts_never_succeed() -> Result<(), TestError> {
    let certain = Probability::try_from_fraction(1, 1)?;
    let run = sample_retry(&certain, 0, 3, 7);
    ensure(run.attempts == vec![None; 3], "no attempt, no success")
}

#[test]
fn an_impossible_retry_never_succeeds() -> Result<(), TestError> {
    let never = Probability::try_from_fraction(0, 1)?;
    let run = sample_retry(&never, 50, 4, 7);
    ensure(run.attempts == vec![None; 4], "probability 0 gives None")?;
    let always = Probability::try_from_fraction(1, 1)?;
    let run = sample_retry(&always, 50, 4, 7);
    ensure(
        run.attempts == vec![Some(1); 4],
        "probability 1 succeeds first",
    )
}

/// With `n` from Hoeffding for `ε = 1/50` and `δ = 10^-9`, the sample mean
/// of "down after 60 steps" misses `1/4` by `ε` or more with probability at
/// most `10^-9`, plus the 60-step mixing error (below `10^-13`) and the
/// threshold rounding (below `60 · 2^-64`). The seed is fixed, so the run is
/// deterministic.
#[test]
fn links_down_fraction_approaches_one_quarter() -> Result<(), TestError> {
    let epsilon = ratio(1, 50);
    let samples = required_samples(&epsilon, &ratio(1, 1_000_000_000))?;
    let chain = links()?;
    let run = sample_chain(
        chain_matrix(&chain)?,
        0,
        STEPS,
        usize::try_from(samples).map_err(|_| TestError::Form)?,
        2026,
    )?;
    let mean = BigRational::new(BigInt::from(run.count(1)), BigInt::from(run.samples()));
    let error = (mean - ratio(1, 4)).abs();
    ensure(
        error < epsilon,
        format!("down fraction misses 1/4 by {error}"),
    )
}

/// One million retries of an attempt that succeeds with probability 3/10.
/// The attempts are geometric with mean `10/3` and variance `70/9`, so by
/// Chebyshev the mean of 10^6 runs misses `10/3` by `1/5` or more with
/// probability at most `(70/9) / (10^6 · (1/5)^2) < 2 · 10^-4`. The
/// first-attempt success fraction is in `[0, 1]`, and by Hoeffding it misses
/// `3/10` by `1/100` or more with probability at most `2 exp(-200) `. A
/// run reaching the 1000-attempt limit has probability `(7/10)^1000`.
#[test]
fn retry_mean_attempts_are_near_ten_thirds() -> Result<(), TestError> {
    let model = retry()?;
    let runs = 1_000_000;
    let run = sample_retry(retry_probability(&model)?, 1000, runs, 2026);
    ensure(run.successes() == run.samples(), "every run succeeds")?;
    let mean = BigRational::new(
        BigInt::from(run.total_attempts()),
        BigInt::from(run.samples()),
    );
    let error = (mean - ratio(10, 3)).abs();
    ensure(
        error < ratio(1, 5),
        format!("mean attempts miss 10/3 by {error}"),
    )?;
    let first = BigRational::new(
        BigInt::from(run.successes_within(1)),
        BigInt::from(run.samples()),
    );
    let first_error = (first - ratio(3, 10)).abs();
    ensure(
        first_error < ratio(1, 100),
        format!("first-attempt fraction misses 3/10 by {first_error}"),
    )
}
