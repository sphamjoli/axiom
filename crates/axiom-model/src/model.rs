//! Models that have passed every static check.

use axiom_core::{Probability, TransitionMatrix};
use num_rational::BigRational;
use num_traits::{One, Zero};

/// One end of a parameter's interval.
///
/// In `]0, 1]` the lower end `0` is open, so `0` itself is excluded, and the
/// upper end `1` is closed, so `1` is included.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bound {
    /// The value of the end, in lowest terms.
    pub value: BigRational,
    /// Whether the value itself belongs to the interval.
    pub closed: bool,
}

/// A declared parameter and the interval it ranges over, such as `a` over
/// `]0, 1[`. The interval is known to contain at least one value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    /// The parameter's name.
    pub name: String,
    /// The lower end of the interval.
    pub lower: Bound,
    /// The upper end of the interval.
    pub upper: Bound,
}

/// An affine weight: a rational constant plus a rational multiple of each
/// parameter.
///
/// With parameters `a` and `b`, the weight `1 - a` has constant `1` and
/// coefficients `[-1, 0]`, and `3/10 + 1/2*b` has constant `3/10` and
/// coefficients `[0, 1/2]`. A weight in a model without parameters is a
/// plain constant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AffineWeight {
    constant: BigRational,
    coefficients: Vec<BigRational>,
}

impl AffineWeight {
    pub(crate) fn zero(parameters: usize) -> Self {
        Self {
            constant: BigRational::zero(),
            coefficients: vec![BigRational::zero(); parameters],
        }
    }

    pub(crate) fn add_constant(&mut self, value: &BigRational) {
        self.constant += value;
    }

    pub(crate) fn add_coefficient(&mut self, parameter: usize, value: &BigRational) {
        if let Some(coefficient) = self.coefficients.get_mut(parameter) {
            *coefficient += value;
        }
    }

    pub(crate) fn add(&mut self, other: &Self) {
        self.constant += &other.constant;
        for (mine, theirs) in self.coefficients.iter_mut().zip(&other.coefficients) {
            *mine += theirs;
        }
    }

    /// The constant part, in lowest terms.
    #[must_use]
    pub fn constant(&self) -> &BigRational {
        &self.constant
    }

    /// The coefficient of each parameter, in declaration order.
    #[must_use]
    pub fn coefficients(&self) -> &[BigRational] {
        &self.coefficients
    }

    /// Whether every coefficient is zero, so the weight does not depend on
    /// any parameter.
    #[must_use]
    pub fn is_constant(&self) -> bool {
        self.coefficients.iter().all(Zero::is_zero)
    }

    /// The index of the parameter the weight is exactly equal to: constant
    /// zero, coefficient one for that parameter and zero for every other.
    /// `a` gives `Some(0)` with parameters `a` and `b`; `1 - a` and `2*a`
    /// give `None`.
    #[must_use]
    fn sole_parameter(&self) -> Option<usize> {
        if !self.constant.is_zero() {
            return None;
        }
        let mut found = None;
        for (index, coefficient) in self.coefficients.iter().enumerate() {
            if coefficient.is_one() && found.is_none() {
                found = Some(index);
            } else if !coefficient.is_zero() {
                return None;
            }
        }
        found
    }

    /// The value of the weight when parameter `i` takes `values[i]`.
    ///
    /// For `1 - a` and `a = 1/10` the value is `9/10`. Missing values count
    /// as zero.
    #[must_use]
    pub fn evaluate(&self, values: &[BigRational]) -> BigRational {
        self.coefficients
            .iter()
            .zip(values)
            .fold(self.constant.clone(), |total, (coefficient, value)| {
                total + coefficient * value
            })
    }
}

/// A finite-state Markov chain that passed every static check.
///
/// `rows[i][j]` is the weight of moving from state `i` to state `j`, with
/// states in declaration order, whatever order the rows were written in. A
/// state absent from a row has weight zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chain {
    /// The chain's name.
    pub name: String,
    /// The parameters, in declaration order.
    pub parameters: Vec<Parameter>,
    /// The state names, in declaration order.
    pub states: Vec<String>,
    /// One row of weights per state.
    pub rows: Vec<Vec<AffineWeight>>,
    /// The exact transition matrix, present when the chain has no
    /// parameters.
    pub matrix: Option<TransitionMatrix>,
}

impl Chain {
    /// The parameters that are the two exit probabilities of a two-state
    /// chain: `a`, the weight of moving from the first state to the second,
    /// and `b`, the weight of moving from the second to the first. `None`
    /// unless the chain has exactly two states and two parameters and each
    /// exit weight is exactly one of them.
    ///
    /// The two are distinct, because the model checker rejects a declared
    /// parameter that is never used, and the stay weights need no check,
    /// because the model checker has shown that every row sums to one.
    #[must_use]
    pub fn two_state_exits(&self) -> Option<(&Parameter, &Parameter)> {
        let [first, second] = self.rows.as_slice() else {
            return None;
        };
        if self.parameters.len() != 2 {
            return None;
        }
        let a = first.get(1)?.sole_parameter()?;
        let b = second.first()?.sole_parameter()?;
        Some((self.parameters.get(a)?, self.parameters.get(b)?))
    }
}

/// A retry recursion that passed every static check: an attempt that
/// succeeds with probability `success` and otherwise repeats itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Retry {
    /// The retry's name.
    pub name: String,
    /// The parameters, in declaration order.
    pub parameters: Vec<Parameter>,
    /// The success probability of one attempt.
    pub success: AffineWeight,
    /// The exact success probability, present when the retry has no
    /// parameters.
    pub probability: Option<Probability>,
}

/// A model that passed every static check: for every parameter assignment in
/// range it denotes a well-formed chain or retry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckedModel {
    /// A finite-state Markov chain.
    Chain(Chain),
    /// A retry recursion.
    Retry(Retry),
}
