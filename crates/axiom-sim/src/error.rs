use thiserror::Error;

/// The reason a simulation or estimate could not be produced.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum SimError {
    /// The model has parameters, so it has no exact probabilities to sample.
    #[error("the model has parameters, so it has no exact probabilities to sample")]
    Parametric,
    /// A state index is outside the chain.
    #[error("state {0} is outside the chain")]
    StateOutOfRange(usize),
    /// The tolerance is not strictly between zero and one.
    #[error("epsilon must lie strictly between zero and one")]
    InvalidEpsilon,
    /// The failure probability is not strictly between zero and one.
    #[error("delta must lie strictly between zero and one")]
    InvalidDelta,
    /// No sample count the search tries meets the requested confidence.
    #[error("no sample count up to the search limit meets the confidence")]
    SamplesUnreachable,
    /// Fewer samples were drawn than the confidence requires.
    #[error("{actual} samples are fewer than the confidence requires")]
    TooFewSamples {
        /// The samples drawn.
        actual: u64,
    },
    /// More hits were counted than samples drawn.
    #[error("more hits than samples")]
    HitsExceedSamples,
}
